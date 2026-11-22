(() => {
  if (window.top !== window) return;
  if (location.hostname !== "twitch.tv" && !location.hostname.endsWith(".twitch.tv")) return;

  const config = window.__twitchClientConfig;
  if (!config) return;

  const cpu = config.settings.resource_cpu_percent;
  const ram = config.settings.resource_ram_mb;
  const label = [
    "Resource limiter",
    cpu ? `${cpu}% CPU` : "CPU unlimited",
    ram ? `${ram} MB RAM` : "RAM unlimited",
  ].join(" | ");
  console.info(`[Twitch Windows Client] ${label}`);
})();
