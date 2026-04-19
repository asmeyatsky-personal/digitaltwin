// ---------------------------------------------------------------------------
// API client for the Rust + .NET backends.
//
// Authentication (`login`, `register`) hits the Rust identity-service on
// `NEXT_PUBLIC_IDENTITY_URL`; everything else still hits the legacy .NET API
// on `NEXT_PUBLIC_API_URL` until each context's Rust port lands per
// MIGRATION.md.
// ---------------------------------------------------------------------------

import { getToken } from "./auth";
import * as identity from "./identity";

const BASE_URL = process.env.NEXT_PUBLIC_API_URL || "http://localhost:8080";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface ApiResponse<T> {
  success: boolean;
  data: T;
  message?: string;
  errors?: string[];
}

export interface LoginResponseRaw {
  success: boolean;
  message?: string;
  token?: string;
  refreshToken?: string;
  expiresIn: number;
  user?: {
    userId: string;
    username: string;
    roles: string[];
  };
}

export interface UserProfile {
  id: string;
  username: string;
  roles: string[];
}

export interface EmotionInsights {
  emotionDistribution: Record<string, number>;
  moodTimeline: { date: string; valence: number }[];
  sessionCount: number;
  averageDurationMinutes: number;
  topEmotions: { emotion: string; count: number; percentage: number }[];
}

export interface ContentReport {
  id: string;
  reporterUserId: string;
  contentType: string;
  contentId: string;
  reason: string;
  description?: string;
  status: string;
  action: string;
  reviewedByUserId?: string;
  reviewNotes?: string;
  createdAt: string;
  reviewedAt?: string;
}

export interface PaginatedReports {
  reports: ContentReport[];
  totalCount: number;
  page: number;
  pageSize: number;
}

export interface ModerationStats {
  totalReports: number;
  pendingCount: number;
  reviewedCount: number;
  actionedCount: number;
  dismissedCount: number;
}

export interface CommunityGroup {
  id: string;
  name: string;
  description: string;
  category: string;
  isModerated: boolean;
  createdByUserId: string;
  memberCount: number;
  createdAt: string;
}

export interface CommunityGroupsResponse {
  groups: CommunityGroup[];
  totalCount: number;
  page: number;
  pageSize: number;
}

export interface CommunityPost {
  id: string;
  groupId: string;
  authorUserId: string;
  title: string;
  content: string;
  isAnonymous: boolean;
  likeCount: number;
  replyCount: number;
  createdAt: string;
}

export interface CommunityPostsResponse {
  posts: CommunityPost[];
  totalCount: number;
  page: number;
  pageSize: number;
}

export interface UserAchievement {
  id: string;
  userId: string;
  key: string;
  title: string;
  description: string;
  iconName: string;
  category: string;
  progress: number;
  requiredCount: number;
  isUnlocked: boolean;
  unlockedAt: string | null;
}

export interface LearningPath {
  id: string;
  title: string;
  description: string;
  category: string;
  estimatedMinutes: number;
  moduleCount: number;
  createdAt: string;
}

export interface LearningPathsResponse {
  paths: LearningPath[];
}

export interface UserLearningProgress {
  id: string;
  userId: string;
  pathId: string;
  currentModuleIndex: number;
  completedModules: string;
  reflectionNotes: string;
  startedAt: string;
  completedAt: string | null;
}

export interface LearningProgressResponse {
  progress: UserLearningProgress[];
}

export interface TherapistProfile {
  id: string;
  userId: string;
  name: string;
  credentials: string;
  bio: string;
  specializations: string;
  availability: string;
  ratePerSession: number;
  isVerified: boolean;
  createdAt: string;
}

export interface TherapistsResponse {
  therapists: TherapistProfile[];
  totalCount: number;
  page: number;
  pageSize: number;
}

export interface ClinicalScreening {
  id: string;
  userId: string;
  type: string;
  responses: string;
  score: number;
  severity: string;
  completedAt: string;
}

// ---------------------------------------------------------------------------
// Generic fetch wrapper
// ---------------------------------------------------------------------------

export async function fetchApi<T>(
  path: string,
  options: RequestInit = {}
): Promise<ApiResponse<T>> {
  const token = getToken();

  const headers: Record<string, string> = {
    "Content-Type": "application/json",
    ...(options.headers as Record<string, string>),
  };

  if (token) {
    headers["Authorization"] = `Bearer ${token}`;
  }

  const res = await fetch(`${BASE_URL}${path}`, {
    ...options,
    headers,
  });

  if (res.status === 401) {
    // Token expired or invalid — redirect handled by caller
    throw new Error("Unauthorized");
  }

  if (!res.ok) {
    const body = await res.json().catch(() => ({}));
    throw new Error(
      (body as ApiResponse<unknown>).message ??
        `Request failed with status ${res.status}`
    );
  }

  return res.json() as Promise<ApiResponse<T>>;
}

// ---------------------------------------------------------------------------
// Auth
// ---------------------------------------------------------------------------

// `username` is treated as the user's email — the Rust identity-service
// authenticates on email. Legacy .NET login accepted "Username" which is now
// mapped to the email field.
export async function login(
  username: string,
  password: string
): Promise<{ token: string; refreshToken: string; user: UserProfile }> {
  const tokens = await identity.authenticate({
    email: username,
    password,
  });

  // Decode the sub claim from the access token to get the user id. Cheaper
  // than a round-trip and the token is already trusted (freshly issued).
  const [, payload] = tokens.accessToken.split(".");
  const { sub } = JSON.parse(
    atob(payload.replace(/-/g, "+").replace(/_/g, "/"))
  ) as { sub: string };

  return {
    token: tokens.accessToken,
    refreshToken: tokens.refreshToken,
    user: { id: sub, username, roles: [] },
  };
}

export async function register(
  email: string,
  password: string
): Promise<{ userId: string }> {
  const out = await identity.registerUser({ email, password });
  return { userId: out.userId };
}

export async function refreshSession(
  refreshToken: string
): Promise<{ token: string; refreshToken: string }> {
  const out = await identity.refreshToken({ refreshToken });
  return { token: out.accessToken, refreshToken: out.refreshToken };
}

export async function logout(refreshToken: string): Promise<void> {
  await identity.revokeToken({ refreshToken });
}

export async function getProfile(userId: string): Promise<UserProfile> {
  const u = await identity.getUser(userId);
  return { id: u.userId, username: u.email, roles: [] };
}

// ---------------------------------------------------------------------------
// Insights
// ---------------------------------------------------------------------------

export async function getEmotionInsights(
  period: "week" | "month" | "all" = "week"
): Promise<ApiResponse<EmotionInsights>> {
  return fetchApi<EmotionInsights>(`/api/insights/emotions?period=${period}`);
}

// ---------------------------------------------------------------------------
// Moderation
// ---------------------------------------------------------------------------

export async function getPendingReports(
  page: number = 1,
  pageSize: number = 20
): Promise<ApiResponse<PaginatedReports>> {
  return fetchApi<PaginatedReports>(
    `/api/moderation/reports?page=${page}&pageSize=${pageSize}`
  );
}

export async function reviewReport(
  id: string,
  action: string,
  notes?: string
): Promise<ApiResponse<ContentReport>> {
  return fetchApi<ContentReport>(`/api/moderation/reports/${id}/review`, {
    method: "POST",
    body: JSON.stringify({ Action: action, Notes: notes }),
  });
}

export async function dismissReport(
  id: string,
  notes?: string
): Promise<ApiResponse<ContentReport>> {
  return fetchApi<ContentReport>(`/api/moderation/reports/${id}/dismiss`, {
    method: "POST",
    body: JSON.stringify({ Notes: notes }),
  });
}

export async function getModerationStats(): Promise<
  ApiResponse<ModerationStats>
> {
  return fetchApi<ModerationStats>("/api/moderation/stats");
}

// ---------------------------------------------------------------------------
// Community
// ---------------------------------------------------------------------------

export async function getCommunityGroups(
  page: number = 1,
  pageSize: number = 20,
  category?: string,
  search?: string
): Promise<ApiResponse<CommunityGroupsResponse>> {
  const params = new URLSearchParams();
  params.set("page", String(page));
  params.set("pageSize", String(pageSize));
  if (category) params.set("category", category);
  if (search) params.set("search", search);
  return fetchApi<CommunityGroupsResponse>(
    `/api/community/groups?${params.toString()}`
  );
}

export async function getCommunityPosts(
  groupId: string,
  page: number = 1,
  pageSize: number = 20
): Promise<ApiResponse<CommunityPostsResponse>> {
  return fetchApi<CommunityPostsResponse>(
    `/api/community/groups/${groupId}/posts?page=${page}&pageSize=${pageSize}`
  );
}

// ---------------------------------------------------------------------------
// Achievements
// ---------------------------------------------------------------------------

export async function getAchievements(): Promise<
  ApiResponse<UserAchievement[]>
> {
  return fetchApi<UserAchievement[]>("/api/achievements");
}

// ---------------------------------------------------------------------------
// Learning
// ---------------------------------------------------------------------------

export async function getLearningPaths(): Promise<
  ApiResponse<LearningPathsResponse>
> {
  return fetchApi<LearningPathsResponse>("/api/learning/paths");
}

export async function getLearningProgress(): Promise<
  ApiResponse<LearningProgressResponse>
> {
  return fetchApi<LearningProgressResponse>("/api/learning/progress");
}

// ---------------------------------------------------------------------------
// Therapy
// ---------------------------------------------------------------------------

export async function getTherapists(
  page: number = 1,
  pageSize: number = 20
): Promise<ApiResponse<TherapistsResponse>> {
  return fetchApi<TherapistsResponse>(
    `/api/therapy/therapists?page=${page}&pageSize=${pageSize}`
  );
}

export async function getScreeningHistory(): Promise<
  ApiResponse<ClinicalScreening[]>
> {
  return fetchApi<ClinicalScreening[]>("/api/therapy/screening/history");
}
