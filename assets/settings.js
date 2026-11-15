(() => {
  const config = window.__twitchClientConfig;
  if (!config || window.top !== window) return;

  const mount = () => {
    if (!document.documentElement) return false;
    if (document.getElementById("twitch-client-settings")) return true;

    const navContainer = document.querySelector(
      '[data-a-target="top-nav-container"]',
    );
    if (!navContainer) return false;

    const host = document.createElement("div");
    host.id = "twitch-client-settings";
    host.style.cssText =
      "all:initial;display:flex;align-items:center;flex:0 0 auto;margin-inline-start:8px;position:relative;z-index:2147483647";
    navContainer.appendChild(host);

    const root = host.attachShadow({ mode: "closed" });
    const style = document.createElement("style");
    style.textContent = `
    :host { all: initial; font: 14px/1.45 "Segoe UI", sans-serif; color: #e6e9ef; }
    * { box-sizing: border-box; }
    button, select { font: inherit; }
    #open, #close, .save {
      color: #fff; background: #9147ff; border: 0; border-radius: 8px;
      padding: 9px 13px; font-weight: 600; cursor: pointer;
    }
    #open { box-shadow: 0 2px 12px #0008; }
    .action { width:100%; margin-top:8px; text-align:left; }
    #streams {
      width:100%; margin-top:8px; padding:8px; color:#e6e9ef;
      background:#313244; border:1px solid #585b70; border-radius:6px;
    }
    #panel {
      width: 340px; max-height: min(75vh, 620px); overflow: auto;
      margin-top: 8px; padding: 18px; border-radius: 12px;
      color: #e6e9ef; background: #181825; border: 1px solid #45475a;
      box-shadow: 0 8px 32px #000a;
    }
    #panel[hidden] { display: none; }
    header { display:flex; align-items:center; justify-content:space-between; margin-bottom:14px; }
    h2 { margin:0; font-size:17px; }
    #close { padding:3px 9px; background:#313244; }
    h3 { margin:18px 0 8px; font-size:13px; color:#cba6f7; }
    label { display:flex; align-items:center; gap:9px; padding:7px 0; cursor:pointer; }
    input { accent-color:#cba6f7; }
    select { width:100%; padding:8px; color:#e6e9ef; background:#313244; border:1px solid #585b70; border-radius:6px; }
    p { margin:8px 0; color:#bac2de; font-size:12px; }
    .notice { margin-top:14px; padding:9px; border-radius:7px; background:#313244; }
    .empty { color:#a6adc8; font-style:italic; }
    `;
    root.appendChild(style);

    const openButton = document.createElement("button");
    openButton.id = "open";
    openButton.type = "button";
    openButton.textContent = "Client settings";
    root.appendChild(openButton);

    const panel = document.createElement("section");
    panel.id = "panel";
    panel.hidden = true;
    panel.setAttribute("role", "dialog");
    panel.setAttribute("aria-label", "Client settings");
    root.appendChild(panel);

    const panelHeader = document.createElement("header");
    const title = document.createElement("h2");
    title.textContent = "Client settings";
    const closeButton = document.createElement("button");
    closeButton.id = "close";
    closeButton.type = "button";
    closeButton.textContent = "Close";
    panelHeader.append(title, closeButton);
    panel.appendChild(panelHeader);

    const addSection = (text) => {
      const heading = document.createElement("h3");
      heading.textContent = text;
      panel.appendChild(heading);
    };
    const send = (message) => {
      if (!window.ipc) {
        window.alert("The client settings bridge is unavailable. Restart the app and try again.");
        return;
      }
      window.ipc.postMessage(JSON.stringify(message));
    };
    const addAction = (text, onClick) => {
      const button = document.createElement("button");
      button.type = "button";
      button.className = "save action";
      button.textContent = text;
      button.addEventListener("click", onClick);
      panel.appendChild(button);
      return button;
    };

    addSection("Themes");
    const themeSelect = document.createElement("select");
    themeSelect.setAttribute("aria-label", "Theme");
    const defaultTheme = document.createElement("option");
    defaultTheme.value = "";
    defaultTheme.textContent = "Twitch default";
    themeSelect.appendChild(defaultTheme);
    for (const theme of config.themes) {
      const option = document.createElement("option");
      option.value = theme;
      option.textContent = theme.replace(/\.css$/i, "").replace(/[-_]/g, " ");
      themeSelect.appendChild(option);
    }
    themeSelect.value = config.settings.theme || "";
    themeSelect.addEventListener("change", () => {
      send({ type: "set_theme", name: themeSelect.value || null });
    });
    panel.appendChild(themeSelect);
    const themeHint = document.createElement("p");
    themeHint.textContent = "Add .css files to the themes folder. Theme changes reload Twitch.";
    panel.appendChild(themeHint);

    addSection("Playback");
    addAction("Picture-in-Picture", () => {
      if (document.pictureInPictureElement) {
        document.exitPictureInPicture().catch((error) => {
          window.alert(`Could not close Picture-in-Picture: ${error.message}`);
        });
        return;
      }
      if (
        !document.pictureInPictureEnabled ||
        typeof HTMLVideoElement.prototype.requestPictureInPicture !== "function"
      ) {
        window.alert("Picture-in-Picture is not available in this Twitch player.");
        return;
      }
      const video = [...document.querySelectorAll("video")]
        .filter(
          (element) =>
            !element.paused &&
            !element.ended &&
            element.readyState >= HTMLMediaElement.HAVE_CURRENT_DATA,
        )
        .map((element) => {
          const bounds = element.getBoundingClientRect();
          return { element, area: bounds.width * bounds.height };
        })
        .filter(({ area }) => area > 0)
        .sort((left, right) => right.area - left.area)[0]?.element;
      if (!video) {
        window.alert("Start a Twitch stream before opening Picture-in-Picture.");
        return;
      }
      video.disablePictureInPicture = false;
      video.requestPictureInPicture().catch((error) => {
        window.alert(`Could not open Picture-in-Picture: ${error.message}`);
      });
    });
    const streamsInput = document.createElement("input");
    streamsInput.id = "streams";
    streamsInput.type = "text";
    streamsInput.placeholder = "channel1, channel2, ...";
    streamsInput.setAttribute("aria-label", "Twitch channels for multi-stream");
    panel.appendChild(streamsInput);
    const streamsHint = document.createElement("p");
    streamsHint.textContent =
      "Enter 2-4 channel names, separated by commas. Only the first stream has audio.";
    panel.appendChild(streamsHint);
    addAction("Open multi-stream", () => {
      const channels = streamsInput.value
        .split(",")
        .map((channel) => channel.trim())
        .filter(Boolean);
      const validName = /^[a-zA-Z0-9_]{1,25}$/;
      if (
        channels.length < 2 ||
        channels.length > 4 ||
        channels.some((channel) => !validName.test(channel)) ||
        new Set(channels.map((channel) => channel.toLowerCase())).size !==
          channels.length
      ) {
        window.alert("Enter 2-4 unique Twitch channel names (letters, numbers, and underscores).");
        return;
      }
      send({ type: "open_multi_stream", channels });
    });
    addAction("Inspect page (F12)", () => {
      send({ type: "open_dev_tools" });
    });

    addSection("Plugins");
    if (config.plugins.length === 0) {
      const empty = document.createElement("p");
      empty.className = "empty";
      empty.textContent = "No plugins found. Add .js files to the plugins folder.";
      panel.appendChild(empty);
    }
    for (const plugin of config.plugins) {
      const label = document.createElement("label");
      const checkbox = document.createElement("input");
      checkbox.type = "checkbox";
      checkbox.checked = config.settings.plugins.includes(plugin);
      checkbox.addEventListener("change", () => {
        send({
          type: "set_plugin",
          name: plugin,
          enabled: checkbox.checked,
        });
      });
      const name = document.createElement("span");
      name.textContent = plugin
        .replace(/\.(?:js|yaml)$/i, "")
        .replace(/[-_]/g, " ");
      label.append(checkbox, name);
      panel.appendChild(label);
    }
    const warning = document.createElement("p");
    warning.className = "notice";
    warning.textContent =
      "Plugins run with access to Twitch pages and your signed-in session. Only enable scripts you trust.";
    panel.appendChild(warning);

    openButton.addEventListener("click", () => {
      panel.hidden = !panel.hidden;
    });
    closeButton.addEventListener("click", () => {
      panel.hidden = true;
    });
    return true;
  };

  if (!mount()) {
    const observer = new MutationObserver(() => {
      if (mount()) observer.disconnect();
    });
    observer.observe(document, { childList: true, subtree: true });
  }
})();
