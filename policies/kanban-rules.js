// Kanban state machine policy
var rules = {
  name: "kanban-rules",
  priority: 10,

  onTick: function() {
    agentdesk.log.info("[kanban-rules] tick");
  },

  onCardTerminal: function(payload) {
    agentdesk.log.info("[kanban-rules] card terminal: " + payload.card_id);
  },

  onSessionStatusChange: function(payload) {
    if (payload.status === "working" && payload.dispatchId) {
      var rows = agentdesk.db.query(
        "SELECT id, status FROM kanban_cards WHERE latest_dispatch_id = ?",
        [payload.dispatchId]
      );
      if (rows.length > 0 && rows[0].status === "requested") {
        agentdesk.log.info("[kanban-rules] promoting card " + rows[0].id + " to in_progress");
      }
    }
  }
};

agentdesk.registerPolicy(rules);
