// Kanban state machine policy
export default {
  name: "kanban-rules",
  priority: 10,

  onSessionStatusChange({ agentId, status, dispatchId }) {
    if (status === "working" && dispatchId) {
      const card = agentdesk.kanban.getByDispatchId(dispatchId);
      if (card && card.status === "requested") {
        agentdesk.kanban.transition(card.id, "in_progress", "agent_started");
      }
    }
  },

  onCardTerminal({ card }) {
    if (card.github_issue_url) {
      agentdesk.github.closeIssue(card.repo_id, card.github_issue_number);
    }
  },
};
