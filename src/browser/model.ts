// ABOUTME: Defines small, deterministic browser rules shared by the UI.
// ABOUTME: Keeps address parsing and tab bounds independent from native code.

export const MAX_TABS = 5;
export const HOME_URL = "https://duckduckgo.com/";

export function normalizeAddress(input: string): string {
  const value = input.trim();
  if (!value) throw new Error("Masukkan alamat atau kata pencarian.");

  if (/^[a-z][a-z\d+.-]*:/i.test(value)) {
    let url: URL;
    try {
      url = new URL(value);
    } catch {
      throw new Error("Hanya alamat HTTPS yang diizinkan.");
    }
    if (url.protocol !== "https:" || !url.hostname) {
      throw new Error("Hanya alamat HTTPS yang diizinkan.");
    }
    return url.toString();
  }

  if (!/\s/.test(value) && value.includes(".")) {
    return new URL(`https://${value}`).toString();
  }

  return `https://duckduckgo.com/?q=${encodeURIComponent(value)}`;
}

export function tabCapacityAvailable(tabCount: number): boolean {
  return tabCount < MAX_TABS;
}

export function reserveTabId(ids: Set<number>, id: number): boolean {
  if (ids.size >= MAX_TABS || ids.has(id)) return false;
  ids.add(id);
  return true;
}

export function releaseTabId(ids: Set<number>, id: number): void {
  ids.delete(id);
}

export function isSleepEligible(activeId: number | null, candidateId: number): boolean {
  return activeId !== candidateId;
}
