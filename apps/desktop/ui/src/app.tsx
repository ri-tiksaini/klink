import { createSignal, For, onCleanup, onMount, Show } from 'solid-js';
import { KlinkRenderer } from './gl/renderer';
import {
  fetchImpactMask,
  fetchNodeDetail,
  fetchPackedEdges,
  fetchPackedNodes,
  fetchScanDiff,
  FileSearchHit,
  NodeDetail,
  pickFolder,
  scanFolder,
  ScanDiff,
  ScanSummary,
  searchFiles,
  searchSymbols,
  SearchHit,
} from './state/engine';

const LANG_META = [
  { id: 0, name: 'Rust', hex: '#F97316' },
  { id: 1, name: 'C', hex: '#60A5FA' },
  { id: 2, name: 'C++', hex: '#818CF8' },
  { id: 3, name: 'Python', hex: '#34D399' },
  { id: 4, name: 'TypeScript', hex: '#38BDF8' },
  { id: 5, name: 'JavaScript', hex: '#FACC15' },
  { id: 6, name: 'Go', hex: '#2DD4BF' },
  { id: 7, name: 'Java', hex: '#FB7185' },
  { id: 8, name: 'Build/Config', hex: '#94A3B8' },
];

export default function App() {
  let canvasRef!: HTMLCanvasElement;
  let overlayRef!: HTMLCanvasElement;
  let searchInputRef!: HTMLInputElement;
  let renderer: KlinkRenderer | undefined;

  const [scanning, setScanning] = createSignal(false);
  const [summary, setSummary] = createSignal<ScanSummary | null>(null);
  const [detail, setDetail] = createSignal<NodeDetail | null>(null);
  const [fps, setFps] = createSignal(60);
  const [frameMs, setFrameMs] = createSignal(0);
  const [clickMs, setClickMs] = createSignal<number | null>(null);

  const [showEdges, setShowEdges] = createSignal(true);
  const [showClusters, setShowClusters] = createSignal(true);
  const [motionOn, setMotionOn] = createSignal(!window.matchMedia('(prefers-reduced-motion: reduce)').matches);
  const [diff, setDiff] = createSignal<ScanDiff | null>(null);
  const [liveMsg, setLiveMsg] = createSignal('');
  const [langMask, setLangMask] = createSignal(0x1FF);
  const [confMask, setConfMask] = createSignal(0x7);

  const [query, setQuery] = createSignal('');
  const [fileHits, setFileHits] = createSignal<FileSearchHit[]>([]);
  const [symHits, setSymHits] = createSignal<SearchHit[]>([]);

  async function handleSelect(id: number) {
    if (id < 0) {
      renderer?.uploadImpactMask(new Uint32Array(256 * 8), -1);
      setDetail(null);
      return;
    }
    const t0 = performance.now();
    const [mask, info] = await Promise.all([fetchImpactMask(id), fetchNodeDetail(id)]);
    renderer?.uploadImpactMask(mask, id);
    setClickMs(performance.now() - t0);
    setDetail(info);
    if (info) setLiveMsg(`Selected ${info.path}, fan-in ${info.fan_in}.`);
  }

  async function rescanCurrent() {
    const cur = summary();
    if (!cur || scanning()) return;
    setScanning(true);
    try {
      const sum = await scanFolder(cur.root);
      const [nodeBuf, edgeBuf] = await Promise.all([fetchPackedNodes(), fetchPackedEdges()]);
      renderer?.uploadGraph(nodeBuf, edgeBuf, sum.clusters);
      setSummary(sum);
      const d = await fetchScanDiff();
      setDiff(d);
      if (d) setLiveMsg(`Rescan diff: +${d.added_edges.length} edges, -${d.removed_edges.length} edges.`);
    } finally {
      setScanning(false);
    }
  }

  async function openAndScanFolder() {
    const selected = await pickFolder();
    if (!selected) return;
    setScanning(true);
    setDetail(null);
    try {
      const sum = await scanFolder(selected);
      const [nodeBuf, edgeBuf] = await Promise.all([fetchPackedNodes(), fetchPackedEdges()]);
      renderer?.uploadGraph(nodeBuf, edgeBuf, sum.clusters);
      setSummary(sum);
      setDiff(await fetchScanDiff());
      setLiveMsg(`Scanned ${sum.node_count} files and ${sum.edge_count} edges across ${sum.cluster_count} folders.`);
    } finally {
      setScanning(false);
    }
  }

  async function onSearchInput(val: string) {
    setQuery(val);
    if (!val.trim() || !summary()) {
      setFileHits([]);
      setSymHits([]);
      return;
    }
    const [fh, sh] = await Promise.all([searchFiles(val), searchSymbols(val)]);
    setFileHits(fh);
    setSymHits(sh);
  }

  function toggleLang(id: number) {
    const next = langMask() ^ (1 << id);
    setLangMask(next);
    renderer?.setLangMask(next);
  }

  function toggleConf(id: number) {
    const next = confMask() ^ (1 << id);
    setConfMask(next);
    renderer?.setConfMask(next);
  }

  onMount(() => {
    renderer = new KlinkRenderer(canvasRef, overlayRef, handleSelect, (f, ms) => {
      setFps(f);
      setFrameMs(ms);
    });
    const onResize = () => renderer?.resize();
    const onKey = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k') {
        e.preventDefault();
        searchInputRef?.focus();
      } else if (e.key === 'Escape') {
        setQuery(''); setFileHits([]); setSymHits([]);
        handleSelect(-1);
      } else if (document.activeElement !== searchInputRef) {
        if (e.key.toLowerCase() === 'r') renderer?.fitAll();
        if (e.key === '+' || e.key === '=') renderer?.zoomBy(1.3);
        if (e.key === '-') renderer?.zoomBy(0.76);
        if (e.key === ' ') {
          e.preventDefault();
          const n = !motionOn();
          setMotionOn(n);
          renderer?.setMotion(n);
        }
      }
    };
    window.addEventListener('resize', onResize);
    window.addEventListener('keydown', onKey);
    onCleanup(() => {
      window.removeEventListener('resize', onResize);
      window.removeEventListener('keydown', onKey);
      renderer?.destroy();
    });
  });

  return (
    <>
      <div id="stage">
        <canvas id="gl" ref={canvasRef} />
        <canvas id="overlay" ref={overlayRef} />
      </div>

      <header class="topbar glass">
        <div class="brand">
          <span class="brand-dot" />
          <span>KLINK</span>
          <button class="btn primary" onClick={openAndScanFolder} disabled={scanning()}>
            {scanning() ? 'Scanning...' : 'Open Folder'}
          </button>
          <Show when={summary()}>
            <button class="btn" onClick={rescanCurrent} disabled={scanning()} title="Rescan & show diff">Rescan / Diff</button>
          </Show>
        </div>

        <div class="search-wrap">
          <input
            ref={searchInputRef}
            type="text"
            value={query()}
            onInput={(e) => onSearchInput(e.currentTarget.value)}
            placeholder="Search files or symbols..."
          />
          <span class="kbd">Ctrl K</span>
          <Show when={fileHits().length > 0 || symHits().length > 0}>
            <div class="search-drop glass">
              <For each={fileHits()}>
                {(f) => (
                  <div class="search-item" onClick={() => {
                    renderer?.flyTo(f.x, f.y, 2.0);
                    handleSelect(f.node_id);
                    setFileHits([]); setSymHits([]); setQuery('');
                  }}>
                    <span>{f.path}</span>
                    <span style="color:var(--accent)">{f.language}</span>
                  </div>
                )}
              </For>
              <For each={symHits()}>
                {(s) => (
                  <div class="search-item" onClick={() => {
                    renderer?.flyTo(s.x, s.y, 2.0);
                    handleSelect(s.file_id);
                    setFileHits([]); setSymHits([]); setQuery('');
                  }}>
                    <span>fn {s.name} ({s.path}:{s.line})</span>
                    <span style="color:var(--impact)">{s.xref_count} refs</span>
                  </div>
                )}
              </For>
            </div>
          </Show>
        </div>

        <div class="controls">
          <button class={`btn ${showClusters() ? 'active' : ''}`} onClick={() => { const v = !showClusters(); setShowClusters(v); renderer?.setShowClusters(v); }}>Folders</button>
          <button class={`btn ${showEdges() ? 'active' : ''}`} onClick={() => { const v = !showEdges(); setShowEdges(v); renderer?.setShowEdges(v); }}>Edges</button>
          <button class={`btn ${motionOn() ? 'active' : ''}`} onClick={() => { const v = !motionOn(); setMotionOn(v); renderer?.setMotion(v); }}>Breathing</button>
          <button class="btn" onClick={() => renderer?.fitAll()}>Fit</button>
        </div>

        <div class="hud">
          <span>FPS <strong class="hud-ok">{fps()}</strong></span>
          <span>FRAME <strong>{frameMs().toFixed(1)}</strong>ms</span>
          <span>CLICK <strong class="hud-ok">{clickMs() !== null ? clickMs()!.toFixed(1) : '--'}</strong>ms</span>
        </div>
      </header>

      <Show when={!summary()}>
        <div class="empty-hero glass">
          <h2 style="margin-bottom:8px;">Open a Repository Folder</h2>
          <p style="color:var(--muted); margin-bottom:16px; line-height:1.5;">
            Klink statically maps all 8 Tier-1 languages into folder-layered WebGL2 clusters with exact blast-radius highlighting.
          </p>
          <button class="btn primary" style="height:36px; padding:0 18px;" onClick={openAndScanFolder}>
            Select Folder...
          </button>
        </div>
      </Show>

      <Show when={summary()}>
        {(s) => (
          <>
            <aside class="left-rail glass">
              <div>
                <div class="rail-title">Folder Clusters ({s().clusters.length})</div>
                <div class="chip-list" style="max-height:190px; overflow-y:auto;">
                  <For each={s().clusters}>
                    {(c) => (
                      <button class="chip" onClick={() => renderer?.flyTo(c.cx, c.cy, Math.max(0.6, 220 / c.radius))}>
                        <span class="chip-l"><span class="chip-label mono">{c.name}</span></span>
                        <span class="cnt">{c.member_count}</span>
                      </button>
                    )}
                  </For>
                </div>
              </div>

              <div>
                <div class="rail-title">Languages</div>
                <div class="chip-list">
                  <For each={LANG_META.filter((l) => (s().lang_counts[l.id] || 0) > 0)}>
                    {(l) => (
                      <button class={`chip ${((langMask() >> l.id) & 1) === 0 ? 'off' : ''}`} onClick={() => toggleLang(l.id)}>
                        <span class="chip-l"><span class="swatch" style={`background:${l.hex}`} />{l.name}</span>
                        <span class="cnt">{s().lang_counts[l.id]}</span>
                      </button>
                    )}
                  </For>
                </div>
              </div>

              <div>
                <div class="rail-title">Edge Confidence</div>
                <div class="chip-list">
                  <button class={`chip ${((confMask() >> 0) & 1) === 0 ? 'off' : ''}`} onClick={() => toggleConf(0)}>
                    <span class="chip-l"><span class="edge-sample" />Static</span>
                    <span class="cnt">{s().static_edges}</span>
                  </button>
                  <button class={`chip ${((confMask() >> 1) & 1) === 0 ? 'off' : ''}`} onClick={() => toggleConf(1)}>
                    <span class="chip-l"><span class="edge-sample heur" />Heuristic</span>
                    <span class="cnt">{s().heuristic_edges}</span>
                  </button>
                  <button class={`chip ${((confMask() >> 2) & 1) === 0 ? 'off' : ''}`} onClick={() => toggleConf(2)}>
                    <span class="chip-l"><span class="edge-sample unres" />Unresolved</span>
                    <span class="cnt">{s().unresolved_count}</span>
                  </button>
                </div>
              </div>
            </aside>

            <div class="nav-pad glass">
              <button class="nav-btn" title="Zoom In" onClick={() => renderer?.zoomBy(1.35)}>+</button>
              <button class="nav-btn" title="Zoom Out" onClick={() => renderer?.zoomBy(0.74)}>−</button>
              <button class="nav-btn" title="Fit All (R)" onClick={() => renderer?.fitAll()}>⊡</button>
            </div>

            <footer class="statusbar glass">
              <div>
                Files: <strong>{s().node_count}</strong> · Edges: <strong>{s().edge_count}</strong> · Folders: <strong>{s().cluster_count}</strong> · Cycles: <strong>{s().cycle_count}</strong> · Symbols: <strong>{s().symbol_count}</strong> · Scan: <strong>{s().duration_ms} ms</strong>
              </div>
              <div>Click node for R32UI glow · Click folder to fly · Esc to clear</div>
            </footer>
          </>
        )}
      </Show>

      <Show when={diff() && (diff()!.added_edges.length > 0 || diff()!.removed_edges.length > 0)}>
        {(df) => (
          <div class="diff-drawer glass">
            <div style="display:flex; justify-content:space-between; align-items:center; margin-bottom:6px;">
              <div class="rail-title" style="margin:0;">Scan Diff (+{df().added_edges.length} / -{df().removed_edges.length})</div>
              <button class="btn" style="height:22px; padding:0 6px;" onClick={() => setDiff(null)}>✕</button>
            </div>
            <For each={df().added_edges}>
              {(e) => <div class="mono" style="color:var(--ok);">+ {e.src} → {e.dst}</div>}
            </For>
            <For each={df().removed_edges}>
              {(e) => <div class="mono" style="color:var(--danger);">− {e.src} → {e.dst}</div>}
            </For>
          </div>
        )}
      </Show>

      <div class="sr-only" aria-live="polite">{liveMsg()}</div>

      <Show when={detail()}>
        {(d) => (
          <aside class="detail glass">
            <div style="display:flex; justify-content:space-between; align-items:flex-start; gap:8px;">
              <div>
                <div class="mono" style="font-weight:700; word-break:break-all;">{d().path}</div>
                <div class="mono" style="color:var(--muted); margin-top:2px;">
                  {d().language} · {d().cluster_name}{d().in_cycle ? ' · SCC Cycle' : ''}
                </div>
              </div>
              <button class="btn" style="height:24px; padding:0 8px;" onClick={() => handleSelect(-1)}>✕</button>
            </div>

            <div class="metrics">
              <div class="mcard"><div class="mval" style="color:var(--impact)">{d().fan_in}</div><div class="mlbl">Fan-In</div></div>
              <div class="mcard"><div class="mval">{d().fan_out}</div><div class="mlbl">Fan-Out</div></div>
              <div class="mcard"><div class="mval">{d().instability.toFixed(2)}</div><div class="mlbl">Instability</div></div>
            </div>

            <div>
              <div class="rail-title">Direct Dependents ({d().direct_dependents.length})</div>
              <ul class="dep-list">
                <For each={d().direct_dependents}>
                  {([id, p]) => (
                    <li class="dep-row mono" onClick={() => handleSelect(id)}>{p}</li>
                  )}
                </For>
                <Show when={d().direct_dependents.length === 0}>
                  <li class="mono" style="color:var(--muted); padding:4px;">Leaf file — no direct dependents.</li>
                </Show>
              </ul>
            </div>

            <Show when={d().symbols.length > 0}>
              <div>
                <div class="rail-title">Defined Symbols ({d().symbols.length})</div>
                <div class="mono" style="color:var(--muted); line-height:1.6;">
                  {d().symbols.slice(0, 14).join(', ')}
                </div>
              </div>
            </Show>
          </aside>
        )}
      </Show>
    </>
  );
}
