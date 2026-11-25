import { invoke } from '@tauri-apps/api/core';

export interface ClusterInfo {
  id: number;
  name: string;
  cx: number;
  cy: number;
  radius: number;
  member_count: number;
}

export interface ScanSummary {
  root: string;
  node_count: number;
  edge_count: number;
  static_edges: number;
  heuristic_edges: number;
  symbol_count: number;
  cycle_count: number;
  cluster_count: number;
  unresolved_count: number;
  duration_ms: number;
  lang_counts: number[];
  clusters: ClusterInfo[];
}

export interface NodeDetail {
  node_id: number;
  path: string;
  language: string;
  cluster_name: string;
  x: number;
  y: number;
  fan_in: number;
  fan_out: number;
  instability: number;
  centrality: number;
  in_cycle: boolean;
  direct_dependents: [number, string][];
  symbols: string[];
}

export interface FileSearchHit {
  node_id: number;
  path: string;
  language: string;
  x: number;
  y: number;
}

export interface SearchHit {
  name: string;
  file_id: number;
  path: string;
  x: number;
  y: number;
  line: number;
  match_kind: string;
  xref_count: number;
}

export async function pickFolder(): Promise<string | null> {
  return invoke<string | null>('pick_folder');
}

export async function scanFolder(path: string): Promise<ScanSummary> {
  return invoke<ScanSummary>('scan_folder', { path });
}

export async function fetchPackedNodes(): Promise<ArrayBuffer> {
  return invoke<ArrayBuffer>('get_packed_nodes');
}

export async function fetchPackedEdges(): Promise<ArrayBuffer> {
  return invoke<ArrayBuffer>('get_packed_edges');
}

export async function fetchImpactMask(nodeId: number): Promise<Uint32Array> {
  const buf = await invoke<ArrayBuffer>('query_impact_mask', { nodeId });
  return new Uint32Array(buf);
}

export async function fetchNodeDetail(nodeId: number): Promise<NodeDetail | null> {
  return invoke<NodeDetail | null>('get_node_detail', { nodeId });
}

export async function searchFiles(query: string): Promise<FileSearchHit[]> {
  return invoke<FileSearchHit[]>('search_files', { query });
}

export async function searchSymbols(query: string): Promise<SearchHit[]> {
  return invoke<SearchHit[]>('search_symbols', { query });
}

export interface EdgeDiffItem {
  src: string;
  dst: string;
}

export interface ScanDiff {
  added_edges: EdgeDiffItem[];
  removed_edges: EdgeDiffItem[];
  added_files: string[];
  removed_files: string[];
}

export async function fetchScanDiff(): Promise<ScanDiff | null> {
  return invoke<ScanDiff | null>('get_scan_diff');
}
