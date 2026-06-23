import type {
  AuditView,
  AuthorView,
  CommitInfo,
  Decision,
  DiffLine,
  FileInfo,
  MultiSigProgress,
  PolicySummary,
  VerificationReport,
} from './types';

async function get<T>(url: string): Promise<T> {
  const res = await fetch(url);
  if (!res.ok) throw await asError(res);
  return (await res.json()) as T;
}

async function post<T>(url: string, body: unknown): Promise<T> {
  const res = await fetch(url, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(body),
  });
  if (!res.ok) throw await asError(res);
  return (await res.json()) as T;
}

async function asError(res: Response): Promise<Error> {
  const body = (await res.json().catch(() => ({}))) as { error?: string };
  return new Error(body.error ?? `${res.status} ${res.statusText}`);
}

export const api = {
  policies: () => get<PolicySummary[]>('/api/policies'),
  info: (id: string) => get<FileInfo>(`/api/policies/${encodeURIComponent(id)}`),
  verify: (id: string) => get<VerificationReport>(`/api/policies/${encodeURIComponent(id)}/verify`),
  rules: (id: string) => get<{ rules: string }>(`/api/policies/${encodeURIComponent(id)}/rules`),
  create: (id: string, rules: string) => post<CommitInfo>('/api/policies', { id, rules }),
  diff: (id: string, proposed: string) =>
    post<DiffLine[]>(`/api/policies/${encodeURIComponent(id)}/diff`, { proposed }),
  commit: (id: string, rules: string, message: string) =>
    post<CommitInfo>(`/api/policies/${encodeURIComponent(id)}/versions`, { rules, message }),
  multisig: (id: string) =>
    get<MultiSigProgress>(`/api/policies/${encodeURIComponent(id)}/multisig`),
  approve: (id: string, author: number) =>
    post<MultiSigProgress>(`/api/policies/${encodeURIComponent(id)}/multisig/approve`, { author }),
  simulate: (id: string, input: Record<string, number>) =>
    post<Decision>(`/api/policies/${encodeURIComponent(id)}/simulate`, { input }),
  audit: (id: string) => get<AuditView>(`/api/policies/${encodeURIComponent(id)}/audit`),
  complianceReport: async (id: string, framework: string): Promise<string> => {
    const res = await fetch(
      `/api/policies/${encodeURIComponent(id)}/compliance?framework=${framework}&format=markdown`,
    );
    if (!res.ok) throw new Error(`${res.status} ${res.statusText}`);
    return res.text();
  },
  registry: () => get<AuthorView[]>('/api/registry'),
  registerAuthor: () => post<AuthorView>('/api/registry/register', {}),
  rotateKey: (author: number) => post<AuthorView>(`/api/registry/${author}/rotate`, {}),
  revokeKey: (author: number, reason: string) =>
    post<AuthorView>(`/api/registry/${author}/revoke`, { reason }),
};

/** URL for the downloadable trusted-JSON registry export. */
export const registryExportUrl = '/api/registry/export';

/** URL to download a policy export in the given format. */
export const exportUrl = (id: string, format: string) =>
  `/api/policies/${encodeURIComponent(id)}/export?format=${format}`;

/** Render a byte array as a lowercase hex string. */
export function hex(bytes: number[]): string {
  return bytes.map((b) => b.toString(16).padStart(2, '0')).join('');
}

/** Short hash form for dense display: first 12 hex chars + ellipsis. */
export function shortHex(bytes: number[]): string {
  const h = hex(bytes);
  return h.length > 12 ? `${h.slice(0, 12)}…` : h;
}
