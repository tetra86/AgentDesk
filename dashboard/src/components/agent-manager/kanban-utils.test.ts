import { describe, expect, it } from "vitest";
import type { GitHubComment } from "../../api";
import { parseGitHubCommentTimeline } from "./kanban-utils";

function makeComment(body: string, author = "itismyfield"): GitHubComment {
  return {
    author: { login: author },
    body,
    createdAt: "2026-03-23T09:00:00Z",
  };
}

describe("parseGitHubCommentTimeline", () => {
  it("리뷰 진행 마커 코멘트를 review 이벤트로 파싱한다", () => {
    const [entry] = parseGitHubCommentTimeline([
      makeComment("🔍 칸반 상태: **review** (카운터모델 리뷰 진행 중)"),
    ]);

    expect(entry).toMatchObject({
      kind: "review",
      status: "reviewing",
      title: "리뷰 진행",
    });
  });

  it("리뷰 피드백 코멘트에서 첫 지적 사항을 요약으로 추출한다", () => {
    const [entry] = parseGitHubCommentTimeline([
      makeComment(`코드 리뷰 결과입니다.

1. **High** — 첫 번째 문제
2. **Medium** — 두 번째 문제`),
    ]);

    expect(entry).toMatchObject({
      kind: "review",
      status: "changes_requested",
      summary: "High — 첫 번째 문제",
    });
    expect(entry.details).toContain("Medium — 두 번째 문제");
  });

  it("리뷰 통과 코멘트를 pass 이벤트로 파싱한다", () => {
    const [entry] = parseGitHubCommentTimeline([
      makeComment("추가 blocking finding은 없습니다. 현재 diff 기준으로 머지를 막을 추가 결함은 확인하지 못했습니다."),
    ]);

    expect(entry).toMatchObject({
      kind: "review",
      status: "passed",
      title: "리뷰 통과",
    });
  });

  it("완료 보고 코멘트를 작업 이력 이벤트로 파싱한다", () => {
    const [entry] = parseGitHubCommentTimeline([
      makeComment(`## #68 완료 보고

### 변경 요약
- something

### 검증
- tests

### DoD
- [x] item`),
    ]);

    expect(entry).toMatchObject({
      kind: "work",
      status: "completed",
      title: "#68 완료 보고",
    });
    expect(entry.details).toEqual(["변경 요약", "검증", "DoD"]);
  });

  it("미분류 코멘트를 general 타입으로 반환한다", () => {
    const [entry] = parseGitHubCommentTimeline([
      makeComment("이건 그냥 일반 코멘트입니다."),
    ]);

    expect(entry).toMatchObject({
      kind: "general",
      status: "comment",
      title: "이건 그냥 일반 코멘트입니다.",
      author: "itismyfield",
    });
  });

  it("빈 코멘트는 무시한다", () => {
    const result = parseGitHubCommentTimeline([makeComment("")]);
    expect(result).toHaveLength(0);
  });

  it("긴 코멘트의 summary를 200자로 잘라낸다", () => {
    const longBody = "A".repeat(300);
    const [entry] = parseGitHubCommentTimeline([makeComment(longBody)]);

    expect(entry.kind).toBe("general");
    expect(entry.summary!.length).toBeLessThanOrEqual(201); // 200 + "…"
  });

  it("PM 결정 코멘트를 pm 타입으로 파싱한다", () => {
    const [entry] = parseGitHubCommentTimeline([
      makeComment(`## PM 결정

- 이 방향으로 진행
- 리스크 수용`),
    ]);

    expect(entry).toMatchObject({
      kind: "pm",
      status: "decision",
      title: "PM 결정",
    });
  });
});
