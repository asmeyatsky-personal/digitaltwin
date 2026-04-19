import { useAuthStore } from "./store";

const API_BASE =
  process.env.EXPO_PUBLIC_API_URL || "http://localhost:8080";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface ApiResponse<T> {
  success: boolean;
  data: T;
  message?: string;
  error?: string;
}

// --- Auth DTOs (match server AuthController.cs) ---

export interface AuthTokens {
  accessToken: string;
  refreshToken: string;
  expiresIn: number;
}

export interface UserProfile {
  id: string;
  username: string;
  roles: string[];
}

export interface LoginRequest {
  Username: string;
  Password: string;
}

export interface RegisterRequest {
  Username: string;
  Email: string;
  Password: string;
  FirstName: string;
  LastName: string;
}

/** Raw shape returned by POST /api/auth/login */
interface LoginResponseRaw {
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

/** Raw shape returned by POST /api/auth/register */
interface RegisterResponseRaw {
  success: boolean;
  message?: string;
  token?: string;
  refreshToken?: string;
  user?: {
    userId: string;
    username: string;
    roles: string[];
  };
  errors?: string[];
}

/** Raw shape returned by POST /api/auth/refresh */
interface TokenRefreshResponseRaw {
  success: boolean;
  message?: string;
  token?: string;
  refreshToken?: string;
  expiresIn: number;
}

// --- Conversation DTOs (match server ConversationController.cs) ---

export interface ConversationStartResponse {
  sessionId: string;
  response: string;
  emotionalTone: string;
  timestamp: string;
}

export interface ConversationMessageRequest {
  conversationId: string;
  message: string;
}

export interface ConversationMessageResponse {
  response: string;
  detectedEmotion: string;
  aiEmotionalTone: string;
  responseTime: string;
  conversationId: string;
}

export interface ConversationMessage {
  id: string;
  content: string;
  response?: string;
  userEmotion: string;
  aiEmotion?: string;
  timestamp: string;
  messageType: string;
}

export interface ConversationHistoryResponse {
  messages: ConversationMessage[];
  totalCount: number;
  page: number;
  pageSize: number;
}

export interface ConversationEndRequest {
  conversationId: string;
  sessionDuration?: string; // TimeSpan serialised as string
}

// --- Legacy types kept for other parts of the app ---

export interface EmotionResult {
  primary: string;
  confidence: number;
  secondary?: string;
  valence: number; // -1 to 1
  arousal: number; // 0 to 1
}

/** @deprecated Use ConversationMessage instead */
export interface Message {
  id: string;
  conversationId: string;
  role: "user" | "assistant";
  content: string;
  emotion?: EmotionResult;
  timestamp: string;
  audioUrl?: string;
}

/** @deprecated Use ConversationStartResponse instead */
export interface Conversation {
  id: string;
  userId: string;
  title: string;
  createdAt: string;
  updatedAt: string;
  messageCount: number;
  lastEmotion?: string;
}

export interface SendMessageRequest {
  conversationId: string;
  message: string;
}

export interface SendMessageResponse {
  response: string;
  detectedEmotion: string;
  aiEmotionalTone: string;
  responseTime: string;
  conversationId: string;
}

export interface EmotionInsights {
  emotionDistribution: Record<string, number>;
  moodTimeline: { date: string; valence: number }[];
  sessionCount: number;
  averageDurationMinutes: number;
  topEmotions: { emotion: string; count: number; percentage: number }[];
}

// Voice types
export interface VoiceInfo {
  id: string;
  name: string;
  language: string;
  gender: string;
  previewUrl?: string;
  isCustom: boolean;
}

export interface VoiceCloneStatus {
  id: string;
  status: "processing" | "ready" | "failed";
  voiceId?: string;
  error?: string;
}

export interface TTSRequest {
  text: string;
  voiceId?: string;
}

// Avatar types
export interface AvatarGenerationResponse {
  id: string;
  status: "queued" | "processing" | "completed" | "failed";
}

export interface AvatarStatus {
  id: string;
  status: "queued" | "processing" | "completed" | "failed";
  avatarUrl?: string;
  thumbnailUrl?: string;
  error?: string;
}

// Emotion (facial) types
export interface FacialEmotionResponse {
  emotion: string;
  confidence: number;
  allEmotions: Record<string, number>;
}

// Subscription types
export type SubscriptionTier = "free" | "plus" | "premium";

export interface SubscriptionTierInfo {
  tier: SubscriptionTier;
  name: string;
  price: number;
  interval: "month";
  features: string[];
  stripePriceId?: string;
}

export interface SubscriptionInfo {
  id: string;
  tier: SubscriptionTier;
  status: "active" | "canceled" | "past_due" | "trialing";
  currentPeriodEnd: string;
  cancelAtPeriodEnd: boolean;
  stripeCustomerId: string;
}

export interface CheckoutSessionResponse {
  sessionId: string;
  clientSecret: string;
  url?: string;
}

// ---------------------------------------------------------------------------
// HTTP helpers
// ---------------------------------------------------------------------------

async function request<T>(
  path: string,
  options: RequestInit = {}
): Promise<ApiResponse<T>> {
  const token = useAuthStore.getState().token;

  const headers: Record<string, string> = {
    "Content-Type": "application/json",
    ...(options.headers as Record<string, string>),
  };

  if (token) {
    headers["Authorization"] = `Bearer ${token}`;
  }

  const res = await fetch(`${API_BASE}${path}`, {
    ...options,
    headers,
  });

  if (res.status === 401) {
    // Attempt a silent refresh
    const refreshed = await silentRefresh();
    if (refreshed) {
      headers["Authorization"] = `Bearer ${useAuthStore.getState().token}`;
      const retry = await fetch(`${API_BASE}${path}`, { ...options, headers });
      return retry.json() as Promise<ApiResponse<T>>;
    }
    useAuthStore.getState().logout();
    throw new Error("Session expired. Please log in again.");
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

async function silentRefresh(): Promise<boolean> {
  const store = useAuthStore.getState();
  const currentToken = store.token;
  const currentRefreshToken = store.refreshToken;
  if (!currentToken || !currentRefreshToken) return false;

  try {
    const res = await fetch(`${API_BASE}/api/auth/refresh`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        Token: currentToken,
        RefreshToken: currentRefreshToken,
      }),
    });

    if (!res.ok) return false;

    const body = (await res.json()) as TokenRefreshResponseRaw;
    if (body.success && body.token && body.refreshToken) {
      useAuthStore.getState().setTokens(body.token, body.refreshToken);
      return true;
    }
    return false;
  } catch {
    return false;
  }
}

async function requestRaw(
  path: string,
  options: RequestInit = {}
): Promise<Response> {
  const token = useAuthStore.getState().token;

  const headers: Record<string, string> = {
    ...(options.headers as Record<string, string>),
  };

  if (token) {
    headers["Authorization"] = `Bearer ${token}`;
  }

  const res = await fetch(`${API_BASE}${path}`, {
    ...options,
    headers,
  });

  if (res.status === 401) {
    const refreshed = await silentRefresh();
    if (refreshed) {
      headers["Authorization"] = `Bearer ${useAuthStore.getState().token}`;
      return fetch(`${API_BASE}${path}`, { ...options, headers });
    }
    useAuthStore.getState().logout();
    throw new Error("Session expired. Please log in again.");
  }

  if (!res.ok) {
    throw new Error(`Request failed with status ${res.status}`);
  }

  return res;
}

async function requestMultipart<T>(
  path: string,
  formData: FormData
): Promise<ApiResponse<T>> {
  const token = useAuthStore.getState().token;

  const headers: Record<string, string> = {};
  if (token) {
    headers["Authorization"] = `Bearer ${token}`;
  }

  const res = await fetch(`${API_BASE}${path}`, {
    method: "POST",
    headers,
    body: formData,
  });

  if (res.status === 401) {
    const refreshed = await silentRefresh();
    if (refreshed) {
      headers["Authorization"] = `Bearer ${useAuthStore.getState().token}`;
      const retry = await fetch(`${API_BASE}${path}`, {
        method: "POST",
        headers,
        body: formData,
      });
      return retry.json() as Promise<ApiResponse<T>>;
    }
    useAuthStore.getState().logout();
    throw new Error("Session expired. Please log in again.");
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
// Auth API
// ---------------------------------------------------------------------------

/**
 * Login — sends {Username, Password} and maps the server response
 * (LoginResponse with PascalCase fields) into the mobile AuthTokens + UserProfile shape.
 */
// Auth calls go to the Rust identity-service. Non-auth API calls still hit
// the legacy .NET API on API_BASE until each bounded context's Rust port
// lands per MIGRATION.md.
import * as identity from "./identity";

function sub(accessToken: string): string {
  const [, payload] = accessToken.split(".");
  return JSON.parse(atob(payload.replace(/-/g, "+").replace(/_/g, "/"))).sub;
}

export async function login(
  username: string,
  password: string
): Promise<ApiResponse<AuthTokens & { user: UserProfile }>> {
  const out = await identity.authenticate({ email: username, password });
  const expiresIn = out.expiresAt
    ? Math.max(
        0,
        Math.floor((new Date(out.expiresAt).getTime() - Date.now()) / 1000)
      )
    : 900;
  return {
    success: true,
    data: {
      accessToken: out.accessToken,
      refreshToken: out.refreshToken,
      expiresIn,
      user: { id: sub(out.accessToken), username, roles: [] },
    },
  };
}

export async function register(
  req: RegisterRequest
): Promise<ApiResponse<AuthTokens & { user: UserProfile }>> {
  // The Rust identity context tracks email + password only. FirstName/LastName
  // and Username move to a separate profile context later in the migration.
  await identity.registerUser({ email: req.Email, password: req.Password });
  // Immediately authenticate to return a token pair in the same response
  // shape the mobile app already expects.
  return login(req.Email, req.Password);
}

export async function refreshToken(
  _accessToken: string,
  refresh: string
): Promise<ApiResponse<AuthTokens>> {
  const out = await identity.refreshToken({ refreshToken: refresh });
  const expiresIn = out.expiresAt
    ? Math.max(
        0,
        Math.floor((new Date(out.expiresAt).getTime() - Date.now()) / 1000)
      )
    : 900;
  return {
    success: true,
    data: {
      accessToken: out.accessToken,
      refreshToken: out.refreshToken,
      expiresIn,
    },
  };
}

export async function logoutRemote(refresh: string): Promise<void> {
  await identity.revokeToken({ refreshToken: refresh });
}

// ---------------------------------------------------------------------------
// Conversation API
// ---------------------------------------------------------------------------

/**
 * Start a new conversation session.
 * Server requires { Message } in the body (ConversationStartRequest).
 * Response is wrapped in ApiResponse<ConversationStartResponse>.
 */
export async function startConversation(
  message: string
): Promise<ApiResponse<ConversationStartResponse>> {
  return request<ConversationStartResponse>("/api/conversation/start", {
    method: "POST",
    body: JSON.stringify({ Message: message }),
  });
}

/**
 * Send a message within an existing conversation.
 * Server expects { ConversationId, Message } (ConversationMessageRequest).
 * Response is wrapped in ApiResponse<ConversationMessageResponse>.
 */
export async function sendMessage(
  conversationId: string,
  message: string
): Promise<ApiResponse<ConversationMessageResponse>> {
  return request<ConversationMessageResponse>("/api/conversation/message", {
    method: "POST",
    body: JSON.stringify({
      conversationId: conversationId,
      message: message,
    } satisfies SendMessageRequest),
  });
}

/**
 * Get conversation history with pagination.
 * Server endpoint: GET /api/conversation/history/{conversationId}?page=&pageSize=
 * Response is wrapped in ApiResponse<ConversationHistoryResponse>.
 */
export async function getConversationHistory(
  conversationId: string,
  page: number = 1,
  pageSize: number = 50
): Promise<ApiResponse<ConversationHistoryResponse>> {
  return request<ConversationHistoryResponse>(
    `/api/conversation/history/${conversationId}?page=${page}&pageSize=${pageSize}`
  );
}

/**
 * End a conversation session.
 * Server endpoint: POST /api/conversation/end
 * There is no DELETE endpoint on the server; use this to close a session.
 */
export async function endConversation(
  conversationId: string,
  sessionDuration?: string
): Promise<ApiResponse<void>> {
  return request<void>("/api/conversation/end", {
    method: "POST",
    body: JSON.stringify({
      conversationId: conversationId,
      sessionDuration: sessionDuration,
    } satisfies ConversationEndRequest),
  });
}

// ---------------------------------------------------------------------------
// Insights API
// ---------------------------------------------------------------------------

export async function getEmotionInsights(
  period: "week" | "month" | "all" = "week"
): Promise<ApiResponse<EmotionInsights>> {
  return request<EmotionInsights>(`/api/insights/emotions?period=${period}`);
}

// ---------------------------------------------------------------------------
// Voice API
// ---------------------------------------------------------------------------

export async function listVoices(): Promise<ApiResponse<VoiceInfo[]>> {
  return request<VoiceInfo[]>("/api/voice/list");
}

export async function getUserVoice(): Promise<ApiResponse<VoiceInfo | null>> {
  return request<VoiceInfo | null>("/api/voice/user/me");
}

export async function textToSpeech(req: TTSRequest): Promise<Blob> {
  const res = await requestRaw("/api/voice/tts", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(req),
  });
  return res.blob();
}

export async function cloneVoice(
  formData: FormData
): Promise<ApiResponse<VoiceCloneStatus>> {
  return requestMultipart<VoiceCloneStatus>("/api/voice/clone", formData);
}

export async function speechToText(
  audioUri: string
): Promise<ApiResponse<{ text: string; language: string }>> {
  const formData = new FormData();
  formData.append("file", {
    uri: audioUri,
    type: "audio/wav",
    name: "recording.wav",
  } as any);
  return requestMultipart<{ text: string; language: string }>(
    "/api/voice/stt",
    formData
  );
}

// ---------------------------------------------------------------------------
// Avatar API
// ---------------------------------------------------------------------------

export async function generateAvatar(
  formData: FormData
): Promise<ApiResponse<AvatarGenerationResponse>> {
  return requestMultipart<AvatarGenerationResponse>(
    "/api/avatar/generate",
    formData
  );
}

export async function getAvatarStatus(
  id: string
): Promise<ApiResponse<AvatarStatus>> {
  return request<AvatarStatus>(`/api/avatar/${id}/status`);
}

export async function getAvatarDownloadUrl(id: string): Promise<string> {
  return `${API_BASE}/api/avatar/${id}/download`;
}

export async function deleteAvatar(
  id: string
): Promise<ApiResponse<void>> {
  return request<void>(`/api/avatar/${id}`, { method: "DELETE" });
}

// ---------------------------------------------------------------------------
// Facial Emotion API
// ---------------------------------------------------------------------------

export async function analyzeEmotion(
  formData: FormData
): Promise<ApiResponse<FacialEmotionResponse>> {
  return requestMultipart<FacialEmotionResponse>(
    "/api/emotion/analyze",
    formData
  );
}

// ---------------------------------------------------------------------------
// Subscription API
// ---------------------------------------------------------------------------

export async function getSubscriptionTiers(): Promise<
  ApiResponse<SubscriptionTierInfo[]>
> {
  return request<SubscriptionTierInfo[]>("/api/subscription/tiers");
}

export async function getSubscription(): Promise<
  ApiResponse<SubscriptionInfo | null>
> {
  return request<SubscriptionInfo | null>("/api/subscription/current");
}

export async function createCheckoutSession(
  tier: SubscriptionTier,
  platform: "web" | "mobile"
): Promise<ApiResponse<CheckoutSessionResponse>> {
  return request<CheckoutSessionResponse>("/api/subscription/checkout", {
    method: "POST",
    body: JSON.stringify({ tier, platform }),
  });
}

export async function cancelSubscription(): Promise<ApiResponse<void>> {
  return request<void>("/api/subscription/cancel", { method: "POST" });
}

// ---------------------------------------------------------------------------
// Check-In API
// ---------------------------------------------------------------------------

export interface CheckInRecord {
  id: string;
  userId: string;
  scheduledAt: string;
  sentAt?: string;
  type: "daily" | "weekly" | "mood_triggered";
  emotionContext?: string;
  response?: string;
  createdAt: string;
}

export interface CheckInSuggestion {
  type: string;
  message: string;
  emotionContext?: string;
  suggestedAt: string;
}

export async function getPendingCheckIns(): Promise<
  ApiResponse<CheckInRecord[]>
> {
  return request<CheckInRecord[]>("/api/checkin/pending");
}

export async function respondToCheckIn(
  id: string,
  response: string
): Promise<ApiResponse<void>> {
  return request<void>(`/api/checkin/${id}/respond`, {
    method: "POST",
    body: JSON.stringify({ response }),
  });
}

export async function evaluateCheckIn(): Promise<
  ApiResponse<CheckInSuggestion | null>
> {
  return request<CheckInSuggestion | null>("/api/checkin/evaluate", {
    method: "POST",
  });
}

// ---------------------------------------------------------------------------
// Notification API
// ---------------------------------------------------------------------------

export async function registerDevice(
  token: string,
  platform: string
): Promise<ApiResponse<void>> {
  return request<void>("/api/notifications/register-device", {
    method: "POST",
    body: JSON.stringify({ token, platform }),
  });
}

export async function unregisterDevice(
  token: string
): Promise<ApiResponse<void>> {
  return request<void>("/api/notifications/unregister-device", {
    method: "DELETE",
    body: JSON.stringify({ token }),
  });
}

// ---------------------------------------------------------------------------
// Personal History / Life Events API
// ---------------------------------------------------------------------------

export type LifeEventCategory =
  | "Career"
  | "Relationship"
  | "Health"
  | "Education"
  | "Milestone"
  | "Loss"
  | "Achievement"
  | "Travel";

export type EmotionType =
  | "Neutral"
  | "Happy"
  | "Sad"
  | "Angry"
  | "Anxious"
  | "Surprised"
  | "Calm"
  | "Excited";

export interface LifeEvent {
  id: string;
  userId: string;
  title: string;
  description: string;
  eventDate: string;
  category: LifeEventCategory;
  emotionalImpact: EmotionType;
  isRecurring: boolean;
  createdAt: string;
}

export interface PersonalContext {
  id: string;
  userId: string;
  culturalBackground: string;
  communicationPreferences: string; // JSON
  importantPeople: string; // JSON list
  values: string; // JSON list
  updatedAt: string;
}

export interface ConversationLifeContext {
  recentEvents: LifeEvent[];
  upcomingEvents: LifeEvent[];
  personalContext: PersonalContext | null;
}

export async function addLifeEvent(event: {
  title: string;
  description: string;
  eventDate: string;
  category: LifeEventCategory;
  emotionalImpact: EmotionType;
  isRecurring: boolean;
}): Promise<ApiResponse<LifeEvent>> {
  return request<LifeEvent>("/api/personal-history/events", {
    method: "POST",
    body: JSON.stringify({
      Title: event.title,
      Description: event.description,
      EventDate: event.eventDate,
      Category: event.category,
      EmotionalImpact: event.emotionalImpact,
      IsRecurring: event.isRecurring,
    }),
  });
}

export async function updateLifeEvent(
  id: string,
  event: {
    title: string;
    description: string;
    eventDate: string;
    category: LifeEventCategory;
    emotionalImpact: EmotionType;
    isRecurring: boolean;
  }
): Promise<ApiResponse<LifeEvent>> {
  return request<LifeEvent>(`/api/personal-history/events/${id}`, {
    method: "PUT",
    body: JSON.stringify({
      Title: event.title,
      Description: event.description,
      EventDate: event.eventDate,
      Category: event.category,
      EmotionalImpact: event.emotionalImpact,
      IsRecurring: event.isRecurring,
    }),
  });
}

export async function deleteLifeEvent(
  id: string
): Promise<ApiResponse<{ deleted: boolean }>> {
  return request<{ deleted: boolean }>(`/api/personal-history/events/${id}`, {
    method: "DELETE",
  });
}

export async function getTimeline(
  start?: string,
  end?: string
): Promise<ApiResponse<LifeEvent[]>> {
  const params = new URLSearchParams();
  if (start) params.set("start", start);
  if (end) params.set("end", end);
  const qs = params.toString();
  return request<LifeEvent[]>(
    `/api/personal-history/timeline${qs ? `?${qs}` : ""}`
  );
}

export async function getUpcomingEvents(
  daysAhead: number = 30
): Promise<ApiResponse<LifeEvent[]>> {
  return request<LifeEvent[]>(
    `/api/personal-history/upcoming?daysAhead=${daysAhead}`
  );
}

export async function getPersonalContext(): Promise<
  ApiResponse<PersonalContext | null>
> {
  return request<PersonalContext | null>("/api/personal-history/context");
}

export async function updatePersonalContext(context: {
  culturalBackground: string;
  communicationPreferences: string;
  importantPeople: string;
  values: string;
}): Promise<ApiResponse<PersonalContext>> {
  return request<PersonalContext>("/api/personal-history/context", {
    method: "PUT",
    body: JSON.stringify({
      CulturalBackground: context.culturalBackground,
      CommunicationPreferences: context.communicationPreferences,
      ImportantPeople: context.importantPeople,
      Values: context.values,
    }),
  });
}

// ---------------------------------------------------------------------------
// Family / Household API
// ---------------------------------------------------------------------------

export type FamilyRole = "Owner" | "Adult" | "Child";

export interface Family {
  id: string;
  name: string;
  createdByUserId: string;
  createdAt: string;
}

export interface FamilyMember {
  id: string;
  familyId: string;
  userId: string;
  role: FamilyRole;
  joinedAt: string;
}

export interface FamilyInvite {
  id: string;
  familyId: string;
  email: string;
  role: FamilyRole;
  inviteCode: string;
  createdAt: string;
  expiresAt: string;
  isAccepted: boolean;
}

export interface FamilyInsights {
  familyId: string;
  memberCount: number;
  emotionDistribution: Record<string, number>;
  overallMood: string;
  periodStart: string;
  periodEnd: string;
}

export interface FamilyWithMembers {
  family: Family;
  members: FamilyMember[];
}

export async function createFamily(
  name: string
): Promise<ApiResponse<Family>> {
  return request<Family>("/api/family", {
    method: "POST",
    body: JSON.stringify({ Name: name }),
  });
}

export async function getFamily(): Promise<
  ApiResponse<FamilyWithMembers | null>
> {
  return request<FamilyWithMembers | null>("/api/family");
}

export async function inviteFamilyMember(
  familyId: string,
  email: string,
  role: FamilyRole
): Promise<ApiResponse<FamilyInvite>> {
  return request<FamilyInvite>(`/api/family/${familyId}/invite`, {
    method: "POST",
    body: JSON.stringify({ Email: email, Role: role }),
  });
}

export async function joinFamily(
  inviteCode: string
): Promise<ApiResponse<FamilyMember>> {
  return request<FamilyMember>("/api/family/join", {
    method: "POST",
    body: JSON.stringify({ InviteCode: inviteCode }),
  });
}

export async function removeFamilyMember(
  familyId: string,
  memberUserId: string
): Promise<ApiResponse<void>> {
  return request<void>(`/api/family/${familyId}/members/${memberUserId}`, {
    method: "DELETE",
  });
}

export async function getFamilyInsights(
  familyId: string
): Promise<ApiResponse<FamilyInsights>> {
  return request<FamilyInsights>(`/api/family/${familyId}/insights`);
}

// ---------------------------------------------------------------------------
// Achievement / Gamification API
// ---------------------------------------------------------------------------

export type AchievementCategory =
  | "Emotional"
  | "Social"
  | "Growth"
  | "Consistency"
  | "Milestone";

export interface AchievementDefinition {
  id: string;
  key: string;
  title: string;
  description: string;
  iconName: string;
  category: AchievementCategory;
  requiredCount: number;
}

export interface UserAchievement {
  id: string;
  userId: string;
  key: string;
  title: string;
  description: string;
  iconName: string;
  category: AchievementCategory;
  progress: number;
  requiredCount: number;
  isUnlocked: boolean;
  unlockedAt: string | null;
}

export async function getAchievements(): Promise<
  ApiResponse<UserAchievement[]>
> {
  return request<UserAchievement[]>("/api/achievements");
}

export async function getMyAchievements(): Promise<
  ApiResponse<UserAchievement[]>
> {
  return request<UserAchievement[]>("/api/achievements/mine");
}

export async function getUnlockedAchievements(): Promise<
  ApiResponse<UserAchievement[]>
> {
  return request<UserAchievement[]>("/api/achievements/unlocked");
}

// ---------------------------------------------------------------------------
// Community Forums & Peer Support API
// ---------------------------------------------------------------------------

export type GroupCategory =
  | "Support"
  | "Interest"
  | "Wellness"
  | "Mindfulness"
  | "Relationships";

export type CommunityRole = "Member" | "Moderator";

export interface CommunityGroup {
  id: string;
  name: string;
  description: string;
  category: GroupCategory;
  isModerated: boolean;
  createdByUserId: string;
  memberCount: number;
  createdAt: string;
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

export interface CommunityReply {
  id: string;
  postId: string;
  authorUserId: string;
  content: string;
  isAnonymous: boolean;
  likeCount: number;
  createdAt: string;
}

export interface CommunityMembership {
  id: string;
  groupId: string;
  userId: string;
  role: CommunityRole;
  joinedAt: string;
}

export interface CommunityGroupsResponse {
  groups: CommunityGroup[];
  totalCount: number;
  page: number;
  pageSize: number;
}

export interface CommunityPostsResponse {
  posts: CommunityPost[];
  totalCount: number;
  page: number;
  pageSize: number;
}

export interface CommunityPostDetailResponse {
  post: CommunityPost;
  replies: CommunityReply[];
  replyCount: number;
}

export async function createCommunityGroup(
  name: string,
  description: string,
  category: GroupCategory
): Promise<ApiResponse<CommunityGroup>> {
  return request<CommunityGroup>("/api/community/groups", {
    method: "POST",
    body: JSON.stringify({ Name: name, Description: description, Category: category }),
  });
}

export async function getCommunityGroups(
  category?: GroupCategory,
  search?: string,
  page: number = 1,
  pageSize: number = 20
): Promise<ApiResponse<CommunityGroupsResponse>> {
  const params = new URLSearchParams();
  if (category) params.set("category", category);
  if (search) params.set("search", search);
  params.set("page", String(page));
  params.set("pageSize", String(pageSize));
  return request<CommunityGroupsResponse>(
    `/api/community/groups?${params.toString()}`
  );
}

export async function getCommunityGroupById(
  groupId: string
): Promise<ApiResponse<CommunityGroup>> {
  return request<CommunityGroup>(`/api/community/groups/${groupId}`);
}

export async function joinCommunityGroup(
  groupId: string
): Promise<ApiResponse<CommunityMembership>> {
  return request<CommunityMembership>(`/api/community/groups/${groupId}/join`, {
    method: "POST",
  });
}

export async function leaveCommunityGroup(
  groupId: string
): Promise<ApiResponse<void>> {
  return request<void>(`/api/community/groups/${groupId}/leave`, {
    method: "POST",
  });
}

export async function createCommunityPost(
  groupId: string,
  title: string,
  content: string,
  isAnonymous: boolean
): Promise<ApiResponse<CommunityPost>> {
  return request<CommunityPost>(`/api/community/groups/${groupId}/posts`, {
    method: "POST",
    body: JSON.stringify({ Title: title, Content: content, IsAnonymous: isAnonymous }),
  });
}

export async function getCommunityPosts(
  groupId: string,
  page: number = 1,
  pageSize: number = 20
): Promise<ApiResponse<CommunityPostsResponse>> {
  return request<CommunityPostsResponse>(
    `/api/community/groups/${groupId}/posts?page=${page}&pageSize=${pageSize}`
  );
}

export async function getCommunityPostById(
  postId: string
): Promise<ApiResponse<CommunityPostDetailResponse>> {
  return request<CommunityPostDetailResponse>(`/api/community/posts/${postId}`);
}

export async function replyToCommunityPost(
  postId: string,
  content: string,
  isAnonymous: boolean
): Promise<ApiResponse<CommunityReply>> {
  return request<CommunityReply>(`/api/community/posts/${postId}/replies`, {
    method: "POST",
    body: JSON.stringify({ Content: content, IsAnonymous: isAnonymous }),
  });
}

export async function likeCommunityPost(
  postId: string
): Promise<ApiResponse<void>> {
  return request<void>(`/api/community/posts/${postId}/like`, {
    method: "POST",
  });
}

export async function likeCommunityReply(
  replyId: string
): Promise<ApiResponse<void>> {
  return request<void>(`/api/community/replies/${replyId}/like`, {
    method: "POST",
  });
}

export async function getMyCommunityGroups(): Promise<
  ApiResponse<{ groups: CommunityGroup[] }>
> {
  return request<{ groups: CommunityGroup[] }>("/api/community/my-groups");
}

export async function getSuggestedCommunityGroups(): Promise<
  ApiResponse<{ groups: CommunityGroup[] }>
> {
  return request<{ groups: CommunityGroup[] }>("/api/community/suggested");
}

// ---------------------------------------------------------------------------
// Content Moderation API
// ---------------------------------------------------------------------------

export type ReportReason =
  | "Harassment"
  | "Spam"
  | "SelfHarm"
  | "Inappropriate"
  | "Misinformation"
  | "Other";

export type ReportStatus = "Pending" | "Reviewed" | "Actioned" | "Dismissed";

export type ModerationAction =
  | "None"
  | "Warning"
  | "ContentRemoved"
  | "UserSuspended"
  | "UserBanned";

export type ModerationContentType = "Post" | "Reply" | "Message";

export interface ContentReport {
  id: string;
  reporterUserId: string;
  contentType: ModerationContentType;
  contentId: string;
  reason: ReportReason;
  description?: string;
  status: ReportStatus;
  action: ModerationAction;
  reviewedByUserId?: string;
  reviewNotes?: string;
  createdAt: string;
  reviewedAt?: string;
}

export async function reportContent(
  contentType: ModerationContentType,
  contentId: string,
  reason: ReportReason,
  description?: string
): Promise<ApiResponse<ContentReport>> {
  return request<ContentReport>("/api/moderation/report", {
    method: "POST",
    body: JSON.stringify({
      ContentType: contentType,
      ContentId: contentId,
      Reason: reason,
      Description: description,
    }),
  });
}

// ---------------------------------------------------------------------------
// Creative Expression Suite API
// ---------------------------------------------------------------------------

export type CreativeWorkType =
  | "Story"
  | "Poem"
  | "Reflection"
  | "Gratitude"
  | "Letter"
  | "FreeWrite";

export interface CreativeWork {
  id: string;
  userId: string;
  type: CreativeWorkType;
  title: string;
  content: string;
  mood: EmotionType;
  isShared: boolean;
  sharedToGroupId?: string;
  createdAt: string;
  updatedAt: string;
}

export interface CreativeWorksResponse {
  works: CreativeWork[];
  totalCount: number;
  page: number;
  pageSize: number;
}

export interface CollaborativeStory {
  id: string;
  roomId: string;
  title: string;
  createdByUserId: string;
  createdAt: string;
}

export interface StoryChapter {
  id: string;
  storyId: string;
  authorUserId: string;
  content: string;
  chapterOrder: number;
  createdAt: string;
}

export interface CollaborativeStoryDetail {
  story: CollaborativeStory;
  chapters: StoryChapter[];
}

export interface CreativePromptResponse {
  prompt: string;
  type: CreativeWorkType;
}

export async function createCreativeWork(
  type: CreativeWorkType,
  title: string,
  content: string,
  mood: EmotionType
): Promise<ApiResponse<CreativeWork>> {
  return request<CreativeWork>("/api/creative/works", {
    method: "POST",
    body: JSON.stringify({ Type: type, Title: title, Content: content, Mood: mood }),
  });
}

export async function getCreativeWorks(
  type?: CreativeWorkType,
  page: number = 1,
  pageSize: number = 20
): Promise<ApiResponse<CreativeWorksResponse>> {
  const params = new URLSearchParams();
  if (type) params.set("type", type);
  params.set("page", String(page));
  params.set("pageSize", String(pageSize));
  return request<CreativeWorksResponse>(
    `/api/creative/works?${params.toString()}`
  );
}

export async function getCreativeWorkById(
  id: string
): Promise<ApiResponse<CreativeWork>> {
  return request<CreativeWork>(`/api/creative/works/${id}`);
}

export async function updateCreativeWork(
  id: string,
  title: string,
  content: string,
  mood: EmotionType
): Promise<ApiResponse<CreativeWork>> {
  return request<CreativeWork>(`/api/creative/works/${id}`, {
    method: "PUT",
    body: JSON.stringify({ Title: title, Content: content, Mood: mood }),
  });
}

export async function deleteCreativeWork(
  id: string
): Promise<ApiResponse<void>> {
  return request<void>(`/api/creative/works/${id}`, {
    method: "DELETE",
  });
}

export async function shareCreativeWork(
  id: string,
  groupId?: string
): Promise<ApiResponse<CreativeWork>> {
  return request<CreativeWork>(`/api/creative/works/${id}/share`, {
    method: "POST",
    body: JSON.stringify({ GroupId: groupId }),
  });
}

export async function getSharedCreativeWorks(
  groupId?: string,
  page: number = 1,
  pageSize: number = 20
): Promise<ApiResponse<CreativeWorksResponse>> {
  const params = new URLSearchParams();
  if (groupId) params.set("groupId", groupId);
  params.set("page", String(page));
  params.set("pageSize", String(pageSize));
  return request<CreativeWorksResponse>(
    `/api/creative/shared?${params.toString()}`
  );
}

export async function generateCreativePrompt(
  type: CreativeWorkType
): Promise<ApiResponse<CreativePromptResponse>> {
  return request<CreativePromptResponse>("/api/creative/prompt", {
    method: "POST",
    body: JSON.stringify({ Type: type }),
  });
}

export async function startCollaborativeStory(
  roomId: string,
  title: string
): Promise<ApiResponse<CollaborativeStory>> {
  return request<CollaborativeStory>("/api/creative/stories", {
    method: "POST",
    body: JSON.stringify({ RoomId: roomId, Title: title }),
  });
}

export async function addStoryChapter(
  storyId: string,
  content: string
): Promise<ApiResponse<StoryChapter>> {
  return request<StoryChapter>(`/api/creative/stories/${storyId}/chapters`, {
    method: "POST",
    body: JSON.stringify({ Content: content }),
  });
}

export async function getCollaborativeStory(
  storyId: string
): Promise<ApiResponse<CollaborativeStoryDetail>> {
  return request<CollaborativeStoryDetail>(`/api/creative/stories/${storyId}`);
}

// ---------------------------------------------------------------------------
// Therapy Marketplace & Clinical Screening API
// ---------------------------------------------------------------------------

export type SessionStatus = "Scheduled" | "Completed" | "Cancelled" | "NoShow";
export type ScreeningType = "PHQ9" | "GAD7" | "PSS10" | "WHO5";
export type ReferralUrgency = "Low" | "Medium" | "High" | "Critical";

export interface TherapistProfile {
  id: string;
  userId: string;
  name: string;
  credentials: string;
  bio: string;
  specializations: string; // JSON array
  availability: string; // JSON
  ratePerSession: number;
  isVerified: boolean;
  createdAt: string;
}

export interface TherapySession {
  id: string;
  therapistId: string;
  clientUserId: string;
  scheduledAt: string;
  durationMinutes: number;
  status: SessionStatus;
  notes?: string;
  createdAt: string;
}

export interface ClinicalScreening {
  id: string;
  userId: string;
  type: ScreeningType;
  responses: string; // JSON array of ints
  score: number;
  severity: string;
  completedAt: string;
}

export interface TherapistReferral {
  id: string;
  userId: string;
  reason: string;
  urgency: ReferralUrgency;
  isAcknowledged: boolean;
  createdAt: string;
}

export interface TherapistsResponse {
  therapists: TherapistProfile[];
  totalCount: number;
  page: number;
  pageSize: number;
}

export interface TherapySessionsResponse {
  sessions: TherapySession[];
  totalCount: number;
  page: number;
  pageSize: number;
}

export interface ScreeningQuestionsResponse {
  type: string;
  questions: string[];
}

export async function getTherapists(
  specialization?: string,
  page: number = 1,
  pageSize: number = 20
): Promise<ApiResponse<TherapistsResponse>> {
  const params = new URLSearchParams();
  if (specialization) params.set("specialization", specialization);
  params.set("page", String(page));
  params.set("pageSize", String(pageSize));
  return request<TherapistsResponse>(
    `/api/therapy/therapists?${params.toString()}`
  );
}

export async function getTherapistById(
  id: string
): Promise<ApiResponse<TherapistProfile>> {
  return request<TherapistProfile>(`/api/therapy/therapists/${id}`);
}

export async function bookTherapySession(
  therapistId: string,
  scheduledAt: string
): Promise<ApiResponse<TherapySession>> {
  return request<TherapySession>("/api/therapy/sessions", {
    method: "POST",
    body: JSON.stringify({ TherapistId: therapistId, ScheduledAt: scheduledAt }),
  });
}

export async function cancelTherapySession(
  sessionId: string
): Promise<ApiResponse<void>> {
  return request<void>(`/api/therapy/sessions/${sessionId}/cancel`, {
    method: "POST",
  });
}

export async function getTherapySessions(
  page: number = 1,
  pageSize: number = 20
): Promise<ApiResponse<TherapySessionsResponse>> {
  return request<TherapySessionsResponse>(
    `/api/therapy/sessions?page=${page}&pageSize=${pageSize}`
  );
}

export async function getScreeningQuestions(
  type: ScreeningType
): Promise<ApiResponse<ScreeningQuestionsResponse>> {
  return request<ScreeningQuestionsResponse>(
    `/api/therapy/screening/${type}/questions`
  );
}

export async function submitScreening(
  type: ScreeningType,
  responses: number[]
): Promise<ApiResponse<ClinicalScreening>> {
  return request<ClinicalScreening>(`/api/therapy/screening/${type}/submit`, {
    method: "POST",
    body: JSON.stringify({ Responses: responses }),
  });
}

export async function getScreeningHistory(): Promise<
  ApiResponse<ClinicalScreening[]>
> {
  return request<ClinicalScreening[]>("/api/therapy/screening/history");
}

export async function getTherapistReferrals(): Promise<
  ApiResponse<TherapistReferral[]>
> {
  return request<TherapistReferral[]>("/api/therapy/referrals");
}

// ---------------------------------------------------------------------------
// Co-Learning & Education API
// ---------------------------------------------------------------------------

export type LearningCategory =
  | "EmotionalIntelligence"
  | "Mindfulness"
  | "Communication"
  | "StressManagement"
  | "Resilience"
  | "SelfCare";

export interface LearningPath {
  id: string;
  title: string;
  description: string;
  category: LearningCategory;
  estimatedMinutes: number;
  moduleCount: number;
  createdAt: string;
}

export interface LearningModule {
  id: string;
  pathId: string;
  title: string;
  content: string;
  exercisePrompt: string;
  order: number;
}

export interface UserLearningProgress {
  id: string;
  userId: string;
  pathId: string;
  currentModuleIndex: number;
  completedModules: string; // JSON array of ints
  reflectionNotes: string; // JSON object
  startedAt: string;
  completedAt: string | null;
}

export interface LearningPathsResponse {
  paths: LearningPath[];
}

export interface LearningPathDetailResponse {
  path: LearningPath;
  modules: LearningModule[];
}

export interface LearningCurrentModuleResponse {
  module: LearningModule | null;
  progress: UserLearningProgress;
}

export interface LearningProgressResponse {
  progress: UserLearningProgress[];
}

export async function getLearningPaths(
  category?: LearningCategory
): Promise<ApiResponse<LearningPathsResponse>> {
  const params = new URLSearchParams();
  if (category) params.set("category", category);
  const qs = params.toString();
  return request<LearningPathsResponse>(
    `/api/learning/paths${qs ? `?${qs}` : ""}`
  );
}

export async function getLearningPathById(
  pathId: string
): Promise<ApiResponse<LearningPathDetailResponse>> {
  return request<LearningPathDetailResponse>(`/api/learning/paths/${pathId}`);
}

export async function startLearningPath(
  pathId: string
): Promise<ApiResponse<UserLearningProgress>> {
  return request<UserLearningProgress>(`/api/learning/paths/${pathId}/start`, {
    method: "POST",
  });
}

export async function getCurrentLearningModule(
  pathId: string
): Promise<ApiResponse<LearningCurrentModuleResponse>> {
  return request<LearningCurrentModuleResponse>(
    `/api/learning/paths/${pathId}/current`
  );
}

export async function completeLearningModule(
  pathId: string,
  reflectionNotes?: string
): Promise<ApiResponse<UserLearningProgress>> {
  return request<UserLearningProgress>(
    `/api/learning/paths/${pathId}/complete-module`,
    {
      method: "POST",
      body: JSON.stringify({ ReflectionNotes: reflectionNotes }),
    }
  );
}

export async function getLearningProgress(): Promise<
  ApiResponse<LearningProgressResponse>
> {
  return request<LearningProgressResponse>("/api/learning/progress");
}

export async function getSuggestedLearningPath(): Promise<
  ApiResponse<LearningPath | null>
> {
  return request<LearningPath | null>("/api/learning/suggested");
}
