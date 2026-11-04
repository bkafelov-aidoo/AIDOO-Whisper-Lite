export function installedMainTarget(targets) {
  if (!Array.isArray(targets)) return null;
  return targets.find(target => {
    if (target?.type !== "page") return false;
    try {
      const url = new URL(target.url);
      return ["http:", "https:"].includes(url.protocol)
        && url.hostname === "tauri.localhost"
        && ["/", "/index.html"].includes(url.pathname)
        && url.searchParams.get("window") !== "overlay";
    } catch { return false; }
  }) ?? null;
}
