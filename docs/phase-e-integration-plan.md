# OpenClaw Swarm — Phase E: Integration Test Plan
# Let's get this shit done.

## E1: Day Seal Story Test (Small)
1. Create task: "Build Day Seal feature"
2. Verify PlanningAgent outputs WorkSize::Story
3. Verify PhaseManager creates 5 default phases
4. Run each phase: Planning → Design → Implementation → Review → Ship
5. Verify artifacts persist (plan.md, design.md, review.json)
6. Verify activity stream logs all events

## E2: Social Tab Rebuild Epic (Large)
1. Create epic: "Rebuild Social Tab"
2. Verify PlanningAgent outputs WorkSize::Epic with 5 stories
3. Verify dependency graph (Story 4 depends on Story 1)
4. Run stories in parallel where possible
5. Verify epic-level progress aggregation

## E3: Error Handling
1. SSE disconnect: verify exponential backoff reconnect
2. API retry: verify 3 retries with backoff
3. Agent crash: verify graceful degradation, error logged, notification sent

## E4: Documentation
1. Write SKILL.md for agent-sdk-dev skill
2. Update README with phased orchestration usage
3. Document API endpoints

## E5: Polish
1. cargo check — must pass
2. cargo build --release — must succeed
3. cargo test — all tests pass
4. cargo clippy — zero warnings
5. Dashboard: all 5 screens render without console errors

## Acceptance Criteria
- [ ] Day Seal story completes end-to-end in < 20 minutes
- [ ] Social Tab epic shows 5 stories with dependency graph
- [ ] SSE auto-reconnect works after disconnect
- [ ] Mock mode toggle works
- [ ] All keyboard shortcuts functional
- [ ] cargo check passes
- [ ] cargo test passes (or has meaningful tests)
- [ ] Dashboard renders in browser without errors
