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
  geographies: Array<"nation" | "state" | "county">;
  period_label: string;
  series_note: string;
  measures: Measure[];
  caveats: Caveat[];
}

export type Flag = "suppressed" | "underreported" | "provisional";

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
}

/** Every dataset dathren knows about, with its measures and caveats. */
export function datasets(): Dataset[];
/** One dataset's description, measures, and caveats. Throws for an unknown id. */
export function info(dataset: string): Dataset;
/** Fetches normalized records straight from the publisher, off the main thread. */
export function fetch(dataset: string, options?: FetchOptions): Promise<Record[]>;
