var triage = {
  name: "triage-rules",
  priority: 300,

  onTick: function() {
    // Check for un-triaged issues (issues without kanban cards)
    // This would be called periodically by the GitHub sync task
  }
};

agentdesk.registerPolicy(triage);
