var pipeline = {
  name: "pipeline",
  priority: 200,

  // Card transition — check if ready cards should enter pipeline
  onCardTransition: function(payload) {
    if (payload.to !== "ready") return;

    // Check if repo has pipeline stages triggered on 'ready'
    var cards = agentdesk.db.query(
      "SELECT repo_id FROM kanban_cards WHERE id = ?",
      [payload.card_id]
    );
    if (cards.length === 0) return;

    var stages = agentdesk.db.query(
      "SELECT id, stage_name, agent_override_id FROM pipeline_stages WHERE repo_id = ? AND trigger_after = 'ready' ORDER BY stage_order ASC LIMIT 1",
      [cards[0].repo_id]
    );
    if (stages.length > 0) {
      agentdesk.db.execute(
        "UPDATE kanban_cards SET pipeline_stage_id = ?, updated_at = datetime('now') WHERE id = ?",
        [stages[0].id, payload.card_id]
      );
      agentdesk.log.info("[pipeline] Card " + payload.card_id + " assigned to stage: " + stages[0].stage_name);
    }
  },

  // Dispatch completed — NO automatic stage advance.
  // Pipeline stage progression is driven ONLY by explicit lifecycle triggers:
  //   - trigger_after='ready'       → onCardTransition (above)
  //   - trigger_after='review_pass' → review-automation.js processVerdict
  // Implementation dispatch completion routes to review (via kanban-rules),
  // and the next stage dispatches only after review passes.
  // This prevents pipeline/review lifecycle conflicts (#110).
  onDispatchCompleted: function(payload) {
    // No-op: stage advance removed. Review-automation handles post-review pipeline progression.
  }
};

agentdesk.registerPolicy(pipeline);
