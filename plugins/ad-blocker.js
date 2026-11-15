(() => {
  "use strict";

  const rules = {
    network: {
      blocked_hosts: [
        "doubleclick.net",
        "googleadservices.com",
        "googlesyndication.com",
        "amazon-adsystem.com",
      ],
    },
    cosmetic: {
      selectors: [
        'iframe[src*="doubleclick.net"]',
        'iframe[src*="googleadservices.com"]',
        'iframe[src*="googlesyndication.com"]',
        "[data-ad-slot]",
        "[data-ad]",
        ".ad-banner",
        ".advertisement",
      ],
    },
    twitch: {
      client_id: "kimne78kx3ncx6brgo4mv6wki5h1ko",
      worker_identifiers: ["amazon-ivs", "wasmworker", "ivs-worker"],
      backup_player_types: [
        "site",
        "popout",
        "frontpage",
        "mobile_web",
        "embed",
        "picture-by-picture",
        "autoplay",
      ],
    },
  };

  const isAdRequest = (value) => {
    let url;
    try {
      url = new URL(value instanceof Request ? value.url : value, location.href);
    } catch {
      return false;
    }

    const hostname = url.hostname.toLowerCase();
    return (
      hostname === "edge.ads.twitch.tv" ||
      rules.network.blocked_hosts.some(
        (host) => {
          const normalizedHost = host.toLowerCase();
          return (
            hostname === normalizedHost ||
            hostname.endsWith(`.${normalizedHost}`)
          );
        },
      )
    );
  };

  const fetch = window.fetch.bind(window);
  window.fetch = (input, init) => {
    if (isAdRequest(input)) {
      return Promise.resolve(new Response(null, { status: 204 }));
    }
    return fetch(input, init);
  };

  const xhrOpen = XMLHttpRequest.prototype.open;
  const xhrSend = XMLHttpRequest.prototype.send;
  XMLHttpRequest.prototype.open = function (method, url, ...args) {
    this.__twitchClientAdRequest = isAdRequest(url);
    return xhrOpen.call(this, method, url, ...args);
  };
  XMLHttpRequest.prototype.send = function (...args) {
    if (!this.__twitchClientAdRequest) return xhrSend.apply(this, args);

    const xhr = this;
    let response = "";
    if (xhr.responseType === "json") response = null;
    else if (xhr.responseType === "arraybuffer") response = new ArrayBuffer(0);
    else if (xhr.responseType === "blob") response = new Blob();
    else if (xhr.responseType === "document") response = null;
    Object.defineProperties(xhr, {
      readyState: { configurable: true, value: XMLHttpRequest.DONE },
      status: { configurable: true, value: 204 },
      statusText: { configurable: true, value: "No Content" },
      response: { configurable: true, value: response },
      responseText: { configurable: true, value: "" },
      responseXML: { configurable: true, value: null },
      getResponseHeader: { configurable: true, value: () => null },
      getAllResponseHeaders: { configurable: true, value: () => "" },
    });
    queueMicrotask(() => {
      for (const type of ["loadstart", "readystatechange", "load", "loadend"]) {
        xhr.dispatchEvent(new Event(type));
      }
    });
  };

  const styleText = `${rules.cosmetic.selectors.join(",\n")} {
    display: none !important;
    visibility: hidden !important;
  }`;

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

// Adapted from My AdBlock's Twitch worker patch.
// Copyright (c) 2026 Jakub L. Licensed under MIT; see
// licenses/my-adblock-LICENSE.txt.
  function workerPatch(rules) {
    const CLIENT_ID = rules.client_id;
    const BACKUP_PLAYER_TYPES = rules.backup_player_types;
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
    const notify = (type) => {
      try {
        channel?.postMessage(type);
      } catch {}
    };
    const textResponse = (response, text) => {
      const headers = new Headers(response.headers);
      for (const header of [
        "content-length",
        "content-encoding",
        "content-range",
        "accept-ranges",
        "content-md5",
        "digest",
        "etag",
        "last-modified",
      ]) {
        headers.delete(header);
      }
      return new Response(text, {
        status: response.status,
        statusText: response.statusText,
        headers,
      });
    };

    const resolveUrl = (value, baseUrl) => {
      try {
        return new URL(value, baseUrl).href;
      } catch {
        return value;
      }
    };

    function parseMaster(text, baseUrl) {
      const lines = text.split(/\r?\n/);
      const streams = [];
      for (let i = 0; i < lines.length; i++) {
        if (!lines[i].startsWith("#EXT-X-STREAM-INF")) continue;
        const resolution = (lines[i].match(/RESOLUTION=(\d+x\d+)/) || [])[1] || "";
        const frameRate = parseFloat(
          (lines[i].match(/FRAME-RATE=([\d.]+)/) || [])[1] || "0",
        );
        const codecs = (lines[i].match(/CODECS="([^"]+)"/) || [])[1] || "";
        let uriIndex = i + 1;
        while (uriIndex < lines.length && lines[uriIndex].startsWith("#")) {
          uriIndex++;
        }
        const uri = lines[uriIndex]?.trim();
        if (uri) {
          streams.push({
            url: resolveUrl(uri, baseUrl),
            resolution,
            frameRate,
            codecs,
          });
          i = uriIndex;
        }
      }
      return streams;
    }

    function codecFamily(codecs) {
      return codecs
        .split(",")
        .map((codec) => codec.trim().split(".")[0])
        .filter(Boolean)
        .sort()
        .join(",");
    }

    function attribute(line, name) {
      const match = line.match(
        new RegExp(`(?:^|[:,])${name}=(?:"([^"]*)"|([^,]*))`),
      );
      return match?.[1] ?? match?.[2] ?? "";
    }

    function parseMediaPlaylist(text, baseUrl) {
      const lines = text.split(/\r?\n/);
      const adRanges = [];
      for (const line of lines) {
        if (
          !line.startsWith("#EXT-X-DATERANGE:") ||
          !/CLASS="twitch-stitched-ad"/i.test(line)
        ) {
          continue;
        }
        const start = Date.parse(attribute(line, "START-DATE"));
        const endDate = Date.parse(attribute(line, "END-DATE"));
        const duration = Number(
          attribute(line, "DURATION") || attribute(line, "PLANNED-DURATION"),
        );
        const end = Number.isFinite(endDate)
          ? endDate
          : Number.isFinite(duration) && duration > 0
            ? start + duration * 1000
            : NaN;
        if (Number.isFinite(start) && Number.isFinite(end)) {
          adRanges.push({ start, end });
        }
      }

      const segments = [];
      let nextDuration = null;
      let nextTitle = "";
      let programTime = null;
      let mapUrl = null;
      let keyState = "METHOD=NONE";
      let hasByteRange = false;
      for (let i = 0; i < lines.length; i++) {
        const line = lines[i];
        if (line.startsWith("#EXT-X-PROGRAM-DATE-TIME:")) {
          const parsed = Date.parse(line.slice(line.indexOf(":") + 1));
          programTime = Number.isFinite(parsed) ? parsed : null;
        } else if (line.startsWith("#EXT-X-MAP:")) {
          const uri = attribute(line, "URI");
          mapUrl = uri ? resolveUrl(uri, baseUrl) : null;
        } else if (line.startsWith("#EXT-X-KEY:")) {
          keyState = line
            .slice("#EXT-X-KEY:".length)
            .replace(
              /URI="([^"]*)"/,
              (_, uri) => `URI="${resolveUrl(uri, baseUrl)}"`,
            );
        } else if (line.startsWith("#EXT-X-BYTERANGE:")) {
          hasByteRange = true;
        } else if (line.startsWith("#EXTINF:")) {
          const value = line.slice("#EXTINF:".length);
          const comma = value.indexOf(",");
          nextDuration = Number.parseFloat(comma < 0 ? value : value.slice(0, comma));
          nextTitle = comma < 0 ? "" : value.slice(comma + 1).trim();
        } else if (nextDuration !== null && line.trim() && !line.startsWith("#")) {
          const inStitchedAdRange =
            programTime !== null &&
            adRanges.some(
              (range) => programTime >= range.start && programTime < range.end,
            );
          const title = nextTitle.toLowerCase();
          segments.push({
            uriIndex: i,
            uri: resolveUrl(line.trim(), baseUrl),
            duration: nextDuration,
            programTime,
            mapUrl,
            keyState,
            hasByteRange,
            isAd:
              (title !== "" && title !== "live") ||
              inStitchedAdRange,
          });
          if (programTime !== null) {
            programTime += nextDuration * 1000;
          }
          nextDuration = null;
          nextTitle = "";
          hasByteRange = false;
        }
      }
      return { lines, segments };
    }

    function replaceAdSegments(text, mainPlaylist, backupSegments) {
      const lines = [...mainPlaylist.lines];
      let replacements = 0;
      for (const ad of mainPlaylist.segments) {
        if (
          !ad.isAd ||
          ad.hasByteRange ||
          ad.programTime === null ||
          !Number.isFinite(ad.duration)
        ) {
          continue;
        }

        let match = null;
        let smallestDelta = Infinity;
        for (const candidate of backupSegments) {
          if (
            candidate.isAd ||
            candidate.hasByteRange ||
            candidate.programTime === null ||
            candidate.mapUrl !== ad.mapUrl ||
            candidate.keyState !== ad.keyState ||
            !Number.isFinite(candidate.duration) ||
            Math.abs(candidate.duration - ad.duration) > 0.5
          ) {
            continue;
          }
          const delta = Math.abs(candidate.programTime - ad.programTime);
          if (delta <= 500 && delta < smallestDelta) {
            match = candidate;
            smallestDelta = delta;
          }
        }

        if (match) {
          lines[ad.uriIndex] = match.uri;
          replacements++;
        }
      }
      return replacements ? lines.join("\n") : text;
    }

    async function getToken(channelName, playerType, signal) {
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
        signal,
      });
      if (!response.ok) return null;
      const json = await response.json();
      return (
        json?.data?.streamPlaybackAccessToken ||
        json?.streamPlaybackAccessToken ||
        null
      );
    }

    async function getBackupMediaUrl(info, playerType, signal) {
      const token = await getToken(info.channel, playerType, signal);
      if (!token) return null;
      const params = new URLSearchParams(info.masterSearch);
      for (const key of ["sig", "token", "player_type", "playerType", "parent_domains"]) {
        params.delete(key);
      }
      params.set("allow_source", params.get("allow_source") || "true");
      params.set("fast_bread", params.get("fast_bread") || "true");
      params.set("p", String(Math.floor(Math.random() * 1e7)));
      params.set("player_backend", "mediaplayer");
      params.set("platform", "web");
      params.set("player_type", playerType);
      params.set("playlist_include_framerate", "true");
      params.set("reassignments_supported", "true");
      params.set("supported_codecs", params.get("supported_codecs") || "avc1");
      params.set("sig", token.signature);
      params.set("token", token.value);
      const response = await realFetch(
        `https://usher.ttvnw.net/api/channel/hls/${info.channel}.m3u8?${params}`,
        { signal },
      );
      if (!response.ok) return null;
      const streams = parseMaster(await response.text(), response.url);
      const selected = streams.find(
        (stream) =>
          stream.resolution === info.resolution &&
          Math.round(stream.frameRate) === Math.round(info.frameRate) &&
          (!info.codecs || !stream.codecs ||
            codecFamily(stream.codecs) === codecFamily(info.codecs)),
      );
      return selected?.url || null;
    }

    async function getBackupSegments(url, signal) {
      const response = await realFetch(url, { signal });
      if (!response.ok) return null;
      const text = await response.text();
      const playlist = parseMediaPlaylist(text, response.url || url);
      const liveSegments = playlist.segments.filter(
        (segment) => !segment.isAd && !segment.hasByteRange,
      );
      if (
        playlist.lines[0]?.replace(/^\uFEFF/, "").trim() !== "#EXTM3U" ||
        !liveSegments.length
      ) {
        return null;
      }
      return playlist.segments;
    }

    function warnBackupUnavailable(info, error) {
      const now = Date.now();
      if (now - (info.lastWarningAt || 0) < 60_000) return;
      info.lastWarningAt = now;
      console.warn(
        "[Twitch Windows Client] No safe alternate live segment is available; leaving the playlist unchanged.",
        error || "",
      );
    }

    function refreshBackupPlaylist(info) {
      const now = Date.now();
      if (
        info.refreshing ||
        now < info.retryAfter ||
        now - info.lastRefreshAt < 1_000
      ) {
        return;
      }

      info.refreshing = true;
      info.lastRefreshAt = now;
      const controller = new AbortController();
      const timeout = setTimeout(() => controller.abort(), 3_000);
      (async () => {
        let lastError = null;
        try {
          if (info.backupUrl) {
            try {
              const segments = await getBackupSegments(
                info.backupUrl,
                controller.signal,
              );
              if (segments) {
                info.backupSegments = segments;
                info.backupCachedAt = Date.now();
                info.retryAfter = 0;
                return;
              }
            } catch (error) {
              lastError = error;
            }
          }
          for (const playerType of BACKUP_PLAYER_TYPES) {
            try {
              const backupUrl = await getBackupMediaUrl(
                info,
                playerType,
                controller.signal,
              );
              if (!backupUrl) continue;
              const segments = await getBackupSegments(
                backupUrl,
                controller.signal,
              );
              if (segments) {
                info.backupUrl = backupUrl;
                info.backupSegments = segments;
                info.backupCachedAt = Date.now();
                info.retryAfter = 0;
                return;
              }
            } catch (error) {
              lastError = error;
            }
          }
          info.retryAfter = Date.now() + 15_000;
          warnBackupUnavailable(info, lastError);
        } catch (error) {
          info.retryAfter = Date.now() + 15_000;
          warnBackupUnavailable(info, error);
        } finally {
          clearTimeout(timeout);
          info.refreshing = false;
        }
      })();
    }

    function replaceAdSegmentsFromBackup(url, text, playlist) {
      const info = mediaInfo.get(url);
      if (!info) return text;

      refreshBackupPlaylist(info);
      if (
        !info.backupSegments ||
        Date.now() - info.backupCachedAt >= 3_000
      ) {
        return text;
      }
      return replaceAdSegments(text, playlist, info.backupSegments);
    }

    self.fetch = async function (input, init) {
      const url = urlOf(input);
      const isMaster =
        url.includes("usher.ttvnw.net/api/") && /\/channel\/hls\//.test(url);
      if (isMaster) {
        const response = await realFetch(input, init);
        if (response.status !== 200) return response;
        const originalResponse = response.clone();
        const text = await response.text();
        const channelName = (url.match(/\/channel\/hls\/([^/.?]+)\.m3u8/) || [])[1];
        if (channelName) {
          const masterSearch = new URL(url).searchParams.toString();
          for (const stream of parseMaster(text, url)) {
            const info = mediaInfo.get(stream.url) || {
              backupUrl: null,
              backupSegments: null,
              backupCachedAt: 0,
              lastRefreshAt: 0,
              lastWarningAt: 0,
              retryAfter: 0,
              refreshing: false,
            };
            Object.assign(info, {
              channel: channelName.toLowerCase(),
              resolution: stream.resolution,
              frameRate: stream.frameRate,
              codecs: stream.codecs,
              masterSearch,
            });
            mediaInfo.set(stream.url, info);
          }
        }
        return originalResponse;
      }

      if (url.includes("ttvnw.net") && url.includes(".m3u8")) {
        const response = await realFetch(input, init);
        if (response.status !== 200) return response;
        const originalResponse = response.clone();
        const text = await response.text();
        const playlist = parseMediaPlaylist(text, response.url || url);
        const hasAdSegments = playlist.segments.some((segment) => segment.isAd);
        if (hasAdSegments) {
          if (!adActive) {
            adActive = true;
            notify("ad-start");
          }
          const rewritten = replaceAdSegmentsFromBackup(url, text, playlist);
          return rewritten === text
            ? originalResponse
            : textResponse(response, rewritten);
        } else if (adActive) {
          adActive = false;
          const info = mediaInfo.get(url);
          if (info) info.backupUrl = null;
          notify("ad-end");
        }
        return originalResponse;
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
    return rules.twitch.worker_identifiers.some((identifier) =>
      text.includes(identifier.toLowerCase()),
    );
  }

  class PatchedWorker extends RealWorker {
    constructor(scriptURL, options) {
      if (options && options.type === "module") {
        super(scriptURL, options);
        return;
      }
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
      const twitchRules = JSON.stringify(rules.twitch);
      const blob = new Blob(
        [`(${workerPatch.toString()})(${twitchRules});\n${source}`],
        { type: "text/javascript" },
      );
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
        banner.textContent = "Ad detected; trying to skip…";
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
