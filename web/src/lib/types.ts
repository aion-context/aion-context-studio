// Mirrors the JSON the studio API returns (studio-core + aion-context types).

export interface PolicySummary {
  id: string;
  file_id: string; // 16-hex (u64 exceeds JS safe ints)
  version_count: number;
  current_version: number;
  valid: boolean;
}

export interface VersionInfo {
  version_number: number;
  author_id: number;
  timestamp: number; // ns since epoch
  message: string;
  rules_hash: number[];
  parent_hash: number[] | null;
}

export interface SignatureInfo {
  version_number: number;
  author_id: number;
  public_key: number[];
  verified: boolean;
  error: string | null;
}

export interface FileInfo {
  file_id: number;
  version_count: number;
  current_version: number;
  versions: VersionInfo[];
  signatures: SignatureInfo[];
}

export interface VerificationReport {
  file_id: number;
  version_count: number;
  structure_valid: boolean;
  integrity_hash_valid: boolean;
  hash_chain_valid: boolean;
  signatures_valid: boolean;
  is_valid: boolean;
  errors: string[];
  temporal_warnings: unknown[];
}

export type DiffTag = 'same' | 'add' | 'del';
export interface DiffLine {
  tag: DiffTag;
  text: string;
}

export interface CommitInfo {
  version: number;
  rules_hash: string;
}

export interface MultiSigProgress {
  version: number;
  threshold: number;
  signers: number[];
  approvers: number[];
  valid_count: number;
  required: number;
  threshold_met: boolean;
  missing: number[];
}

export interface EpochView {
  epoch: number;
  public_key: string;
  created_at_version: number;
  status: 'active' | 'rotated' | 'revoked';
  detail: string;
}

export interface AuthorView {
  author_id: number;
  epochs: EpochView[];
}

export interface TraceStep {
  rule_id: string;
  matched: boolean;
  note: string;
}

export interface Decision {
  decision: string;
  matched_rule: string | null;
  trace: TraceStep[];
}

export interface AuditEntryView {
  index: number;
  timestamp: number;
  author_id: number;
  action: string;
  detail: string;
  hash: string;
}

export interface AuditView {
  entries: AuditEntryView[];
}
