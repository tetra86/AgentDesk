var pipeline = {
  name: "pipeline",
  priority: 200,

  onReviewVerdict: function(payload) {
    if (payload.verdict === "pass") {
      // Check if repo has pipeline stages
      var stages = agentdesk.db.query(
        "SELECT * FROM pipeline_stages WHERE repo_id = ? AND trigger_after = 'review_pass' ORDER BY stage_order ASC LIMIT 1",
        [payload.repo_id]
      );
      if (stages.length > 0) {
        agentdesk.log.info("[pipeline] Starting post-review stage: " + stages[0].stage_name);
        // Would create dispatch for the stage
      } else {
        // No pipeline — card goes to done
        agentdesk.db.execute(
          "UPDATE kanban_cards SET status = 'done', updated_at = datetime('now') WHERE id = ?",
          [payload.card_id]
        );
        agentdesk.log.info("[pipeline] No stages, card → done: " + payload.card_id);
      }
    }
  }
};

agentdesk.registerPolicy(pipeline);
