// ABOUTME: Renders the trusted browser chrome above isolated page webviews.
// ABOUTME: Manages bounded tabs, sleeping, privacy controls, and capture self-test UX.

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { HOME_URL, MAX_TABS, isSleepEligible, normalizeAddress, tabCapacityAvailable } from "../browser/model";

type Tab = {
  id: number;
  title: string;
  url: string;
  loading: boolean;
  sleeping: boolean;
};

type PageState = {
  id: number;
  url: string;
  title: string;
  loading: boolean;
};

type CaptureSelfTest = {
  platform: string;
  protected: boolean;
  affinity: number;
  detail: string;
};

const NEW_TAB_TITLE = "Tab baru";

function messageOf(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function displayHost(url: string): string {
  try {
    return new URL(url).hostname.replace(/^www\./, "");
  } catch {
    return NEW_TAB_TITLE;
  }
}

export default function MainApp() {
  const nextId = useRef(1);
  const booted = useRef(false);
  const creatingTab = useRef(false);
  const sleepTimers = useRef(new Map<number, number>());
  const [tabs, setTabs] = useState<Tab[]>([]);
  const [activeId, setActiveId] = useState<number | null>(null);
  const activeIdRef = useRef<number | null>(null);
  const [address, setAddress] = useState(HOME_URL);
  const [opacity, setOpacity] = useState(94);
  const [clickThrough, setClickThrough] = useState(false);
  const [error, setError] = useState("");
  const [showPrivacy, setShowPrivacy] = useState(false);
  const [selfTest, setSelfTest] = useState<CaptureSelfTest | null>(null);
  const [checks, setChecks] = useState<Record<string, boolean>>({});

  const activeTab = useMemo(
    () => tabs.find((tab) => tab.id === activeId) ?? null,
    [activeId, tabs],
  );

  useEffect(() => {
    activeIdRef.current = activeId;
  }, [activeId]);

  const armSleep = useCallback((id: number) => {
    const previous = sleepTimers.current.get(id);
    if (previous) window.clearTimeout(previous);
    const timer = window.setTimeout(async () => {
      if (!isSleepEligible(activeIdRef.current, id)) {
        sleepTimers.current.delete(id);
        return;
      }
      try {
        const slept = await invoke<boolean>("browser_sleep_tab", { id });
        if (slept) {
          setTabs((current) => current.map((tab) => (tab.id === id ? { ...tab, sleeping: true } : tab)));
        }
      } catch (cause) {
        setError(messageOf(cause));
      } finally {
        sleepTimers.current.delete(id);
      }
    }, 120_000);
    sleepTimers.current.set(id, timer);
  }, []);

  const cancelSleep = useCallback((id: number) => {
    const timer = sleepTimers.current.get(id);
    if (timer) window.clearTimeout(timer);
    sleepTimers.current.delete(id);
  }, []);

  const activateTab = useCallback(
    async (id: number, sourceTabs: Tab[] = tabs) => {
      const target = sourceTabs.find((tab) => tab.id === id);
      if (!target) return;
      try {
        if (activeId !== null && activeId !== id) armSleep(activeId);
        cancelSleep(id);
        await invoke("browser_activate_tab", { id, url: target.url });
        setTabs((current) => current.map((tab) => (tab.id === id ? { ...tab, sleeping: false } : tab)));
        setActiveId(id);
        setAddress(target.url);
        setError("");
      } catch (cause) {
        setError(messageOf(cause));
      }
    },
    [activeId, armSleep, cancelSleep, tabs],
  );

  const createTab = useCallback(
    async (rawUrl = HOME_URL) => {
      if (creatingTab.current) return;
      if (!tabCapacityAvailable(tabs.length)) {
        setError(`Maksimal ${MAX_TABS} tab.`);
        return;
      }
      creatingTab.current = true;
      const id = nextId.current++;
      try {
        const url = normalizeAddress(rawUrl);
        const tab: Tab = { id, title: NEW_TAB_TITLE, url, loading: true, sleeping: false };
        if (activeId !== null) armSleep(activeId);
        await invoke("browser_create_tab", { id, url });
        const nextTabs = [...tabs, tab];
        setTabs(nextTabs);
        setActiveId(id);
        setAddress(url);
        setError("");
      } catch (cause) {
        setError(messageOf(cause));
      } finally {
        creatingTab.current = false;
      }
    },
    [activeId, armSleep, tabs],
  );

  useEffect(() => {
    if (booted.current) return;
    booted.current = true;
    void createTab();
  }, [createTab]);

  useEffect(() => {
    const unlistenPage = listen<PageState>("browser:page-state", ({ payload }) => {
      setTabs((current) =>
        current.map((tab) =>
          tab.id === payload.id
            ? {
                ...tab,
                url: payload.url || tab.url,
                title: payload.title || tab.title,
                loading: payload.loading,
              }
            : tab,
        ),
      );
      if (payload.id === activeIdRef.current && payload.url) setAddress(payload.url);
    });
    const unlistenClick = listen<boolean>("overlay:click-through", ({ payload }) => {
      setClickThrough(payload);
    });
    const resize = () => void invoke("browser_resize_pages");
    window.addEventListener("resize", resize);
    return () => {
      void unlistenPage.then((fn) => fn());
      void unlistenClick.then((fn) => fn());
      window.removeEventListener("resize", resize);
    };
  }, []);

  useEffect(() => () => {
    sleepTimers.current.forEach((timer) => window.clearTimeout(timer));
  }, []);

  const navigate = async () => {
    if (!activeTab) return;
    try {
      const url = normalizeAddress(address);
      if (activeTab.sleeping) {
        await invoke("browser_activate_tab", { id: activeTab.id, url });
      } else {
        await invoke("browser_navigate", { id: activeTab.id, url });
      }
      setTabs((current) =>
        current.map((tab) =>
          tab.id === activeTab.id ? { ...tab, url, loading: true, sleeping: false } : tab,
        ),
      );
      setAddress(url);
      setError("");
    } catch (cause) {
      setError(messageOf(cause));
    }
  };

  const closeTab = async (id: number) => {
    cancelSleep(id);
    try {
      await invoke("browser_close_tab", { id });
      const remaining = tabs.filter((tab) => tab.id !== id);
      setTabs(remaining);
      if (id === activeId) {
        const replacement = remaining[remaining.length - 1];
        if (replacement) await activateTab(replacement.id, remaining);
        else {
          setActiveId(null);
          setAddress(HOME_URL);
        }
      }
    } catch (cause) {
      setError(messageOf(cause));
    }
  };

  const changeOpacity = async (value: number) => {
    setOpacity(value);
    try {
      await invoke("set_window_opacity", { percent: value });
    } catch (cause) {
      setError(messageOf(cause));
    }
  };

  const toggleClickThrough = async () => {
    try {
      const enabled = !clickThrough;
      await invoke("set_click_through", { enabled });
      setClickThrough(enabled);
    } catch (cause) {
      setError(messageOf(cause));
    }
  };

  const runSelfTest = async () => {
    try {
      const result = await invoke<CaptureSelfTest>("capture_self_test");
      setSelfTest(result);
      setShowPrivacy(true);
      setError("");
    } catch (cause) {
      setError(messageOf(cause));
    }
  };

  return (
    <main className="app-shell">
      <header className="browser-chrome">
        <div className="tabs" aria-label="Tab browser">
          {tabs.map((tab) => (
            <button
              className={`tab ${tab.id === activeId ? "active" : ""}`}
              key={tab.id}
              onClick={() => void activateTab(tab.id)}
              title={tab.url}
            >
              <span className="tab-status">{tab.sleeping ? "z" : tab.loading ? "·" : ""}</span>
              <span>{tab.title === NEW_TAB_TITLE ? displayHost(tab.url) : tab.title}</span>
              <span
                className="tab-close"
                role="button"
                aria-label="Tutup tab"
                onClick={(event) => {
                  event.stopPropagation();
                  void closeTab(tab.id);
                }}
              >
                ×
              </span>
            </button>
          ))}
          <button className="icon-button add-tab" onClick={() => void createTab()} disabled={tabs.length >= MAX_TABS} title={`Tab baru (${tabs.length}/${MAX_TABS})`}>
            +
          </button>
          <div className="drag-region" data-tauri-drag-region />
          <button className={`shield ${selfTest?.protected ? "verified" : ""}`} onClick={() => void runSelfTest()} title="Tes perlindungan capture">
            <span className="shield-mark" />
            {selfTest?.protected ? "Protected" : "Test capture"}
          </button>
        </div>

        <form
          className="toolbar"
          onSubmit={(event) => {
            event.preventDefault();
            void navigate();
          }}
        >
          <button type="button" className="icon-button" onClick={() => activeId !== null && void invoke("browser_history", { id: activeId, delta: -1 })} title="Kembali">‹</button>
          <button type="button" className="icon-button" onClick={() => activeId !== null && void invoke("browser_history", { id: activeId, delta: 1 })} title="Maju">›</button>
          <button type="button" className="icon-button reload" onClick={() => activeId !== null && void invoke("browser_reload", { id: activeId })} title="Muat ulang">↻</button>
          <div className="address-wrap">
            <span className="lock">https</span>
            <input
              aria-label="Alamat atau pencarian"
              value={address}
              onChange={(event) => setAddress(event.target.value)}
              onFocus={(event) => event.currentTarget.select()}
              spellCheck={false}
            />
          </div>
          <label className="opacity-control" title="Transparansi jendela">
            <span>{opacity}%</span>
            <input type="range" min="30" max="100" value={opacity} onChange={(event) => void changeOpacity(Number(event.target.value))} />
          </label>
          <button type="button" className={`text-button ${clickThrough ? "active" : ""}`} onClick={() => void toggleClickThrough()} title="Ctrl+Shift+X untuk keluar dari click-through">
            Click-through
          </button>
          <button type="button" className="icon-button menu-button" onClick={() => setShowPrivacy((value) => !value)} title="Privasi">•••</button>
        </form>
        {error && <div className="error-bar" role="alert">{error}</div>}
      </header>

      {tabs.length === 0 && (
        <section className="empty-state">
          <div className="brand-mark">V</div>
          <h1>Veil Browser</h1>
          <p>Browser overlay privat untuk Windows.</p>
          <button className="primary-button" onClick={() => void createTab()}>Buka tab</button>
        </section>
      )}

      {clickThrough && <div className="click-banner">Click-through aktif · tekan Ctrl+Shift+X untuk mengontrol browser</div>}

      {showPrivacy && (
        <aside className="privacy-panel">
          <div className="panel-heading">
            <div>
              <p className="eyebrow">CAPTURE SELF-TEST</p>
              <h2>{selfTest?.protected ? "Proteksi OS aktif" : "Perlu diperiksa"}</h2>
            </div>
            <button className="icon-button" onClick={() => setShowPrivacy(false)}>×</button>
          </div>
          <p className="status-detail">{selfTest?.detail ?? "Jalankan tes sebelum mulai berbagi layar."}</p>
          <dl>
            <div><dt>Platform</dt><dd>{selfTest?.platform ?? "—"}</dd></div>
            <div><dt>Affinity</dt><dd>{selfTest ? `0x${selfTest.affinity.toString(16).padStart(8, "0")}` : "—"}</dd></div>
            <div><dt>Profil</dt><dd>Sementara · tanpa extension</dd></div>
          </dl>
          <p className="check-title">Periksa preview screen-share:</p>
          {['Zoom', 'Google Meet', 'Microsoft Teams', 'Discord', 'OBS'].map((app) => (
            <label className="check-row" key={app}>
              <input type="checkbox" checked={Boolean(checks[app])} onChange={(event) => setChecks((current) => ({ ...current, [app]: event.target.checked }))} />
              <span>{app}</span>
            </label>
          ))}
          <p className="warning-copy">Ctrl+Shift+H sembunyikan · Ctrl+Shift+J pulihkan · Ctrl+Shift+X click-through.</p>
          <p className="warning-copy">Tidak ada API universal untuk membuktikan hasil capture pihak ketiga. Pastikan jendela tidak terlihat di preview setiap aplikasi.</p>
          <button className="primary-button full" onClick={() => void runSelfTest()}>Ulangi tes OS</button>
        </aside>
      )}
    </main>
  );
}
