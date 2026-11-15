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
