(() => {
  const adHosts = [
    "doubleclick.net",
    "googleadservices.com",
    "googlesyndication.com",
    "amazon-adsystem.com",
  ];

  const isAdRequest = (value) => {
    let url;
    try {
      url = new URL(value instanceof Request ? value.url : value, location.href);
    } catch {
      return false;
    }

    const hostname = url.hostname.toLowerCase();
    return adHosts.some(
      (host) => hostname === host || hostname.endsWith(`.${host}`),
    );
  };

  const fetch = window.fetch.bind(window);
  window.fetch = (input, init) => {
    if (isAdRequest(input)) {
      return Promise.reject(new TypeError("Blocked by the local ad-blocker plugin"));
    }
    return fetch(input, init);
  };

  const styleText = `
    iframe[src*="doubleclick.net"],
    iframe[src*="googleadservices.com"],
    iframe[src*="googlesyndication.com"],
    [data-ad-slot],
    [data-ad],
    .ad-banner,
    .advertisement {
      display: none !important;
      visibility: hidden !important;
    }
  `;

  const installStyle = () => {
    if (document.getElementById("twitch-client-ad-blocker")) return;

    const style = document.createElement("style");
    style.id = "twitch-client-ad-blocker";
    style.textContent = styleText;
    (document.head || document.documentElement)?.appendChild(style);
  };

  new MutationObserver(installStyle).observe(document, {
    childList: true,
    subtree: true,
  });
  installStyle();
})();

// Adapted from My AdBlock's Twitch worker patch.
// Copyright (c) 2026 Jakub L. Licensed under MIT; see
// licenses/my-adblock-LICENSE.txt.
(() => {
  "use strict";

  function workerPatch() {
    const CLIENT_ID = "kimne78kx3ncx6brgo4mv6wki5h1ko";
    const BACKUP_PLAYER_TYPES = ["embed", "popout", "autoplay"];
    const AD_MARKERS = [
      "stitched-ad",
      "X-TV-TWITCH-AD",
      "twitch-stitched-ad",
      "twitch-maf-ad",
      "twitch-ad-quartile",
    ];
    const TOKEN_QUERY =
      'query PlaybackAccessToken_Template($login: String!, $isLive: Boolean!, $vodID: ID!, $isVod: Boolean!, $playerType: String!) {' +
      ' streamPlaybackAccessToken(channelName: $login, params: {platform: "web", playerBackend: "mediaplayer", playerType: $playerType}) @include(if: $isLive) { value signature __typename }' +
      ' videoPlaybackAccessToken(id: $vodID, params: {platform: "web", playerBackend: "mediaplayer", playerType: $playerType}) @include(if: $isVod) { value signature __typename } }';

    const realFetch = self.fetch.bind(self);
    const mediaInfo = new Map();
    let adActive = false;
    let channel = null;
    try {
      channel = new BroadcastChannel("my-adblock-twitch");
    } catch {}

    const urlOf = (input) =>
      (typeof input === "string"
        ? input
        : input instanceof URL
          ? input.href
          : input && input.url) || "";
    const hasAds = (text) => AD_MARKERS.some((marker) => text.includes(marker));
    const notify = (type) => {
      try {
        channel?.postMessage(type);
      } catch {}
    };

    function parseMaster(text) {
      const lines = text.split("\n");
      const streams = [];
      for (let i = 0; i < lines.length - 1; i++) {
        if (!lines[i].startsWith("#EXT-X-STREAM-INF")) continue;
        const resolution = (lines[i].match(/RESOLUTION=(\d+x\d+)/) || [])[1] || "";
        const frameRate = parseFloat(
          (lines[i].match(/FRAME-RATE=([\d.]+)/) || [])[1] || "0",
        );
        const url = lines[i + 1].trim();
        if (url.startsWith("http")) streams.push({ url, resolution, frameRate });
      }
      return streams;
    }

    function stripAds(text) {
      const lines = text.split("\n");
      const output = [];
      for (let i = 0; i < lines.length; i++) {
        const line = lines[i];
        if (line.startsWith("#EXTINF") && !line.includes(",live")) {
          i++;
          continue;
        }
        if (line.startsWith("#EXT-X-TWITCH-PREFETCH")) continue;
        if (line.startsWith("#EXT-X-DATERANGE") && hasAds(line)) continue;
        output.push(line);
      }
      return output.join("\n");
    }

    async function getToken(channelName, playerType) {
      const response = await realFetch("https://gql.twitch.tv/gql", {
        method: "POST",
        headers: {
          "Client-ID": CLIENT_ID,
          "Content-Type": "application/json",
        },
        body: JSON.stringify({
          operationName: "PlaybackAccessToken_Template",
          query: TOKEN_QUERY,
          variables: {
            isLive: true,
            login: channelName,
            isVod: false,
            vodID: "",
            playerType,
          },
        }),
      });
      if (!response.ok) return null;
      const json = await response.json();
      return json?.data?.streamPlaybackAccessToken || null;
    }

    async function getBackupMediaUrl(info, playerType) {
      const token = await getToken(info.channel, playerType);
      if (!token) return null;
      const params = new URLSearchParams({
        allow_source: "true",
        fast_bread: "true",
        p: String(Math.floor(Math.random() * 1e7)),
        player_backend: "mediaplayer",
        playlist_include_framerate: "true",
        reassignments_supported: "true",
        supported_codecs: "avc1",
        sig: token.signature,
        token: token.value,
      });
      const response = await realFetch(
        `https://usher.ttvnw.net/api/channel/hls/${info.channel}.m3u8?${params}`,
      );
      if (!response.ok) return null;
      const streams = parseMaster(await response.text());
      if (!streams.length) return null;
      const same =
        streams.find(
          (stream) =>
            stream.resolution === info.resolution &&
            Math.round(stream.frameRate) === Math.round(info.frameRate),
        ) || streams.find((stream) => stream.resolution === info.resolution);
      return (same || streams[0]).url;
    }

    async function tryPlaylist(url) {
      try {
        const response = await realFetch(url);
        if (!response.ok) return null;
        const text = await response.text();
        return hasAds(text) ? null : text;
      } catch {
        return null;
      }
    }

    async function replaceAdPlaylist(url, text) {
      const info = mediaInfo.get(url);
      if (info) {
        if (info.backupUrl) {
          const clean = await tryPlaylist(info.backupUrl);
          if (clean) return clean;
        }
        for (const playerType of BACKUP_PLAYER_TYPES) {
          try {
            const backupUrl = await getBackupMediaUrl(info, playerType);
            if (!backupUrl) continue;
            const clean = await tryPlaylist(backupUrl);
            if (clean) {
              info.backupUrl = backupUrl;
              return clean;
            }
          } catch {}
        }
      }
      return stripAds(text);
    }

    self.fetch = async function (input, init) {
      const url = urlOf(input);
      if (url.includes("usher.ttvnw.net/api/channel/hls/")) {
        const response = await realFetch(input, init);
        if (response.status !== 200) return response;
        const text = await response.text();
        const channelName = (url.match(/\/hls\/([^/.?]+)\.m3u8/) || [])[1];
        if (channelName) {
          for (const stream of parseMaster(text)) {
            mediaInfo.set(stream.url, {
              channel: channelName.toLowerCase(),
              resolution: stream.resolution,
              frameRate: stream.frameRate,
            });
          }
        }
        return new Response(text, {
          status: response.status,
          statusText: response.statusText,
          headers: response.headers,
        });
      }

      if (url.includes("ttvnw.net") && url.includes(".m3u8")) {
        const response = await realFetch(input, init);
        if (response.status !== 200) return response;
        let text = await response.text();
        if (hasAds(text)) {
          if (!adActive) {
            adActive = true;
            notify("ad-start");
          }
          text = await replaceAdPlaylist(url, text);
        } else if (adActive) {
          adActive = false;
          const info = mediaInfo.get(url);
          if (info) info.backupUrl = null;
          notify("ad-end");
        }
        return new Response(text, {
          status: response.status,
          statusText: response.statusText,
          headers: response.headers,
        });
      }

      return realFetch(input, init);
    };
  }

  const RealWorker = window.Worker;
  if (!RealWorker || RealWorker.__myAdblock) return;

  function readSync(url) {
    try {
      const xhr = new XMLHttpRequest();
      xhr.open("GET", url, false);
      xhr.send();
      return xhr.status === 200 || xhr.status === 0 ? xhr.responseText : null;
    } catch {
      return null;
    }
  }

  function isPlayerWorker(absUrl, source) {
    const text = (absUrl + " " + (source || "")).toLowerCase();
    return (
      text.includes("amazon-ivs") ||
      text.includes("wasmworker") ||
      text.includes("ivs-worker")
    );
  }

  class PatchedWorker extends RealWorker {
    constructor(scriptURL, options) {
      let absUrl = "";
      try {
        absUrl = new URL(String(scriptURL), location.href).href;
      } catch {}
      const source = absUrl.startsWith("blob:")
        ? readSync(absUrl)
        : `importScripts(${JSON.stringify(absUrl)});`;

      if (!source || !isPlayerWorker(absUrl, source)) {
        super(scriptURL, options);
        return;
      }
      const blob = new Blob([`(${workerPatch.toString()})();\n${source}`], {
        type: "text/javascript",
      });
      super(URL.createObjectURL(blob), options);
    }
  }
  PatchedWorker.__myAdblock = true;
  window.Worker = PatchedWorker;

  let banner = null;
  let channel = null;
  try {
    channel = new BroadcastChannel("my-adblock-twitch");
  } catch {}
  if (channel) {
    channel.onmessage = (message) => {
      const container =
        document.querySelector(".video-player__container") ||
        document.querySelector("[data-a-target='video-player']");
      if (message.data === "ad-start" && container && !banner) {
        banner = document.createElement("div");
        banner.textContent = "Ad blocker: skipping ad break…";
        banner.style.cssText =
          "position:absolute;top:10px;left:10px;z-index:9999;padding:4px 8px;" +
          "background:rgba(0,0,0,.7);color:#fff;font:12px sans-serif;border-radius:4px;pointer-events:none";
        container.appendChild(banner);
      } else if (message.data === "ad-end" && banner) {
        banner.remove();
        banner = null;
      }
    };
  }
})();
