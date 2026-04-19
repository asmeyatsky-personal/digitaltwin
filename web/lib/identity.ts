// Client for the Rust identity-service (`backend/services/identity-service`).
// Targets the JSON REST adapter so browsers don't need a gRPC-Web runtime.
// Request/response shapes match the Protobuf-generated TS types.

import {
  AuthenticateRequest,
  AuthenticateResponse,
  GetUserResponse,
  RefreshTokenRequest,
  RefreshTokenResponse,
  RegisterUserRequest,
  RegisterUserResponse,
  RevokeTokenRequest,
  RevokeTokenResponse,
} from "./contracts/digitaltwin/identity/v1/identity";

const IDENTITY_URL =
  process.env.NEXT_PUBLIC_IDENTITY_URL ?? "http://localhost:8080";

async function post<Req, Res>(path: string, body: Req): Promise<Res> {
  const res = await fetch(`${IDENTITY_URL}${path}`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });
  if (!res.ok) {
    const err = await res.json().catch(() => ({ error: `http ${res.status}` }));
    throw new Error((err as { error?: string }).error ?? `http ${res.status}`);
  }
  return res.json() as Promise<Res>;
}

export async function registerUser(
  req: RegisterUserRequest
): Promise<RegisterUserResponse> {
  return post<RegisterUserRequest, RegisterUserResponse>(
    "/v1/auth/register",
    req
  );
}

export async function authenticate(
  req: AuthenticateRequest
): Promise<AuthenticateResponse> {
  return post<AuthenticateRequest, AuthenticateResponse>(
    "/v1/auth/authenticate",
    req
  );
}

export async function refreshToken(
  req: RefreshTokenRequest
): Promise<RefreshTokenResponse> {
  return post<RefreshTokenRequest, RefreshTokenResponse>(
    "/v1/auth/refresh",
    req
  );
}

export async function revokeToken(
  req: RevokeTokenRequest
): Promise<RevokeTokenResponse> {
  return post<RevokeTokenRequest, RevokeTokenResponse>(
    "/v1/auth/revoke",
    req
  );
}

export async function getUser(userId: string): Promise<GetUserResponse> {
  const res = await fetch(
    `${IDENTITY_URL}/v1/users/${encodeURIComponent(userId)}`
  );
  if (!res.ok) {
    const err = await res.json().catch(() => ({ error: `http ${res.status}` }));
    throw new Error((err as { error?: string }).error ?? `http ${res.status}`);
  }
  return res.json() as Promise<GetUserResponse>;
}
