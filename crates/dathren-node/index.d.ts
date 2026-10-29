export interface Measure {
  id: string;
  label: string;
  /** Upstream classification code, e.g. an ICD-10 range. */
  code: string | null;
  unit: string;
}

export interface Caveat {
  title: string;
  detail: string;
}

export interface Dataset {
  id: string;
  title: string;
  publisher: string;
  summary: string;
  source_url: string;
  update_cadence: string;
  topics: string[];
  geographies: Array<"nation" | "state" | "county" | "country">;
  period_label: string;
  series_note: string;
  measures: Measure[];
  caveats: Caveat[];
}

export type Flag = "suppressed" | "underreported" | "provisional" | "modeled";

export interface Record {
  dataset: string;
  measure: string;
  /** `US`, a state abbreviation, or a 5-digit county FIPS code. */
  geo: string;
  geo_name: string | null;
  /** `2026-05`, `2025-Q3`, or `2024`. */
  period: string;
  /** `null` when suppressed upstream, never zero. */
  value: number | null;
  /** The publisher's own correction for known reporting gaps. */
  adjusted_value: number | null;
  ci_low: number | null;
  ci_high: number | null;
  flags: Flag[];
  note: string | null;
}

export interface FetchOptions {
  geo?: string;
  measure?: string | string[];
  since?: number;
  /** Emit a `DathrenWarning` about suppressed and modeled values. Defaults to true. */
  warn?: boolean;
}

export interface Citation {
  apa: string;
  bibtex: string;
  text: string;
  /** How to cite dathren itself. */
  dathren: string;
}

export interface Total {
  /** Sum of the reported values. */
  reported: number;
  /** `reported` plus the smallest value each suppressed cell could hold. */
  low: number;
  /** `reported` plus the largest value each suppressed cell could hold. */
  high: number;
  reported_count: number;
  suppressed_count: number;
}

/** Every dataset dathren knows about, with its measures and caveats. */
export function datasets(): Dataset[];
/** One dataset's description, measures, and caveats. Throws for an unknown id. */
export function info(dataset: string): Dataset;
/** Fetches normalized records straight from the publisher, off the main thread. */
export function fetch(dataset: string, options?: FetchOptions): Promise<Record[]>;
/** Citations for a dataset. `accessed` is `YYYY-MM-DD` and defaults to today. */
export function cite(dataset: string, options?: { accessed?: string }): Citation;
/** Each place's newest value; skips never-published rows but keeps suppressed ones. */
export function latest(rows: Record[]): Record[];
/** Sums one count measure with suppressed cells as a range. Throws for rates or mixed measures. */
export function total(rows: Record[], options?: { suppressed?: [number, number] }): Total;
/** Turns counts into rates per `per` residents (default 100,000) using Census population. */
export function perCapita(rows: Record[], options?: { per?: number }): Promise<Record[]>;
