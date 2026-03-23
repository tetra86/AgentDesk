import test from "node:test";
import assert from "node:assert/strict";

import type { Agent, DispatchedSession } from "../types";
import {
  applySessionOverlay,
  deriveDispatchedAsAgents,
  deriveSubAgents,
} from "./office-session-overlay.ts";

function makeAgent(overrides: Partial<Agent> = {}): Agent {
  return {
    id: "project-agentdesk",
    name: "AgentDesk",
    name_ko: "AgentDesk",
    department_id: "engineering",
    avatar_emoji: "🤖",
    personality: null,
    status: "idle",
    stats_tasks_done: 0,
    stats_xp: 0,
    stats_tokens: 0,
    created_at: 0,
    ...overrides,
  };
}

function makeSession(overrides: Partial<DispatchedSession> = {}): DispatchedSession {
  return {
    id: "session-1",
    session_key: "mac-mini:AgentDesk-codex-adk-cdx-t1485506232256168011",
    name: "adk-cdx-t1485506232256168011",
    department_id: "engineering",
    linked_agent_id: "project-agentdesk",
    provider: "codex",
    model: null,
    status: "working",
    session_info: "리뷰 중",
    sprite_number: null,
    avatar_emoji: "🤖",
    stats_xp: 0,
    tokens: 10,
    connected_at: 0,
    last_seen_at: 0,
    thread_channel_id: "1485506232256168011",
    ...overrides,
  };
}

test("linked thread session overlays parent agent and stays out of dispatched staff", () => {
  const agent = makeAgent();
  const session = makeSession();

  const overlaid = applySessionOverlay([agent], [session]);
  const subAgents = deriveSubAgents([session]);
  const dispatched = deriveDispatchedAsAgents([session]);

  assert.equal(overlaid[0].status, "working");
  assert.equal(overlaid[0].activity_source, "agentdesk");
  assert.equal(overlaid[0].current_thread_channel_id, "1485506232256168011");
  assert.equal(subAgents.length, 1);
  assert.equal(subAgents[0].parentAgentId, "project-agentdesk");
  assert.equal(dispatched.length, 0);
});

test("multiple working sessions: newest by timestamp thread_channel_id wins", () => {
  const agent = makeAgent();
  const newerSession = makeSession({
    id: "session-2",
    session_info: "rework 중",
    thread_channel_id: "9999999999",
    last_seen_at: 200,
    connected_at: 100,
  });
  const olderSession = makeSession({
    id: "session-1",
    session_info: "리뷰 중",
    thread_channel_id: "1111111111",
    last_seen_at: 50,
    connected_at: 10,
  });
  // Regardless of array order, the session with higher timestamp wins
  // Test with newer first (WS prepend order)
  const overlaid = applySessionOverlay([agent], [newerSession, olderSession]);
  assert.equal(overlaid[0].current_thread_channel_id, "9999999999");
  assert.equal(overlaid[0].session_info, "rework 중");
  assert.equal(overlaid[0].agentdesk_working_count, 2);

  // Test with older first (bootstrap ORDER BY id)
  const overlaidReverse = applySessionOverlay([agent], [olderSession, newerSession]);
  assert.equal(overlaidReverse[0].current_thread_channel_id, "9999999999");
  assert.equal(overlaidReverse[0].session_info, "rework 중");
  assert.equal(overlaidReverse[0].agentdesk_working_count, 2);
});

test("direct channel session still overlays agent without requiring a thread id", () => {
  const agent = makeAgent();
  const session = makeSession({
    name: "adk-cdx",
    session_key: "mac-mini:AgentDesk-codex-adk-cdx",
    thread_channel_id: null,
  });

  const overlaid = applySessionOverlay([agent], [session]);

  assert.equal(overlaid[0].status, "working");
  assert.equal(overlaid[0].current_thread_channel_id, null);
});
