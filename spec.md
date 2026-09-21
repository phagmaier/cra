# Project Spec: Internally Gated Learning in a Continuous Recurrent Agent

**Working project name:** Learning When to Learn  
**Document:** `spec.md`  
**Version:** 0.1 - implementation and experimental specification  
**Prepared:** September 20, 2026 (America/Los_Angeles)  
**Implementation assumption:** a Rust simulator and command-line runner; analysis can be written in Python. The equations and experimental contracts are language-independent.  
**Status:** proposed research design, not an implemented or experimentally validated system.

> Build a small, continuously running recurrent nervous system that learns from delayed consequences using local synaptic traces. Then ask whether internally generated signals can regulate **when** and **where** learning occurs, improving adaptation without unnecessarily damaging stable associations.

---

## How to use this document

Work through the milestones in Section 16 in order. Read Sections 1-10 before implementing the main agent. Keep the contracts in Sections 5, 8, and 9 open while writing the simulation loop. Use the test inventory in Section 17 as implementation tickets. Do not begin a large evolutionary run until the ungated local learner passes its own diagnostics.

This specification deliberately separates three kinds of statement:

- **Required contract:** behavior needed for results to mean what the experiment claims.
- **Starting choice:** a concrete proposed default, not a known optimal hyperparameter.
- **Research hypothesis:** a possibility that the experiments may support or reject.

All numerical defaults, proposed benchmarks, and success thresholds below are engineering choices unless explicitly attributed to a source. They are not performance promises. The literature provides foundations and related methods; it does not establish that this particular combination will work or that the research question is novel.

The first implementation uses an explicitly measured stochastic perturbation rule. This makes credit assignment easier to verify than immediately implementing a biologically motivated rule that must infer its own perturbations. It preserves the earlier project's intended direction: local lifetime learning, persistent recurrence, internal modulation, and an evolutionary outer loop. Removing access to the perturbation is a later experiment, not an unacknowledged biological claim.

### Contents

1. [Research objective and scope](#1-research-objective-and-scope)
2. [Minimum viable result and non-goals](#2-minimum-viable-result-and-non-goals)
3. [System architecture](#3-system-architecture)
4. [Notation and time conventions](#4-notation-and-time-conventions)
5. [Environment and information boundary](#5-environment-and-information-boundary)
6. [Neuron dynamics and action interface](#6-neuron-dynamics-and-action-interface)
7. [Local credit assignment](#7-local-credit-assignment)
8. [Internal learning gates](#8-internal-learning-gates)
9. [Exact tick and event ordering](#9-exact-tick-and-event-ordering)
10. [Initialization, bounds, and resets](#10-initialization-bounds-and-resets)
11. [Inherited parameters and lifetime memory](#11-inherited-parameters-and-lifetime-memory)
12. [Evolutionary search](#12-evolutionary-search)
13. [Baselines and fair comparisons](#13-baselines-and-fair-comparisons)
14. [Evaluation and statistical analysis](#14-evaluation-and-statistical-analysis)
15. [Causal interventions](#15-causal-interventions)
16. [Implementation milestones](#16-implementation-milestones)
17. [Test inventory and numerical checks](#17-test-inventory-and-numerical-checks)
18. [Repository and interface design](#18-repository-and-interface-design)
19. [Configuration profiles](#19-configuration-profiles)
20. [Logging, replay, and checkpoints](#20-logging-replay-and-checkpoints)
21. [Diagnostics and troubleshooting](#21-diagnostics-and-troubleshooting)
22. [Compute planning](#22-compute-planning)
23. [Extensions after the core experiment](#23-extensions-after-the-core-experiment)
24. [Interpreting results and bounding claims](#24-interpreting-results-and-bounding-claims)
25. [First implementation checklist](#25-first-implementation-checklist)
26. [Related work and source notes](#26-related-work-and-source-notes)

---

## 1. Research objective and scope

### 1.1 Main question

**Can a recurrent agent's own internal dynamics generate targeted plasticity gates that improve its response to genuine environmental changes while reducing interference from misleading feedback?**

The agent must learn several cue-action associations. Some associations change during its lifetime; others remain stable. Feedback sometimes contradicts the underlying association. The agent is not told which associations are stable, how noisy feedback is, or when a change happens.

The challenge is not merely to choose the correct action. It is to regulate how strongly new experience changes the system's stored behavior.

### 1.2 Hypotheses to test separately

**H1 - lifetime plasticity:** local connection updates improve acquisition or adaptation beyond what the same inherited network can do without those updates.

**H2 - temporal gating:** an internally generated global learning gate improves the adaptation-versus-noise trade-off over a well-tuned fixed learning rate.

**H3 - spatial targeting:** different gates for different receiving neurons provide additional benefit beyond a learned global gate.

**H4 - mechanism:** any benefit depends on the timing or spatial assignment of modulation, not just on a lower effective learning rate, extra network capacity, or a favorable optimization budget.

**H5 - transfer:** the effect survives new initial associations, new event schedules, and at least one held-out environmental condition.

Do not combine these into one vague claim that the system is intelligent. Each hypothesis has different controls and can fail independently.

### 1.3 What is meant by brain-inspired

The project includes persistent recurrent activity, leaky integration, local synaptic state, delayed learning signals, slower internal dynamics, and inherited versus acquired parameters. These are abstractions, not a claim to replicate a biological brain.

The first version uses signed continuous activities, explicit simulation ticks, an externally supplied scalar outcome, a deliberately engineered action interface, and locally accessible exploratory perturbations. It does not enforce realistic anatomy, neurotransmitter chemistry, exact spike timing, or biological energy constraints.

### 1.4 Relationship to prior work

Reward-driven recurrent learning, eligibility traces, neuromodulation, and evolving plasticity already have precedents. Relevant starting points include Miconi's delayed-reward recurrent learning, Backpropamine's differentiable neuromodulation, and evolutionary discovery of plasticity rules in recurrent spiking networks. [R2], [R4], [R5]

The potential contribution here is a controlled investigation of a specific trade-off and mechanism. A publication-level novelty claim requires a more comprehensive related-work review than this implementation document provides.

---

## 2. Minimum viable result and non-goals

### 2.1 Minimum viable scientific system

The first complete system should:

1. Run through thousands of choices without resetting its recurrent state between choices.
2. Improve on initially unknown cue-action associations through lifetime interaction.
3. Adapt when an association changes, without being told that it changed.
4. Produce measurable local synaptic updates from delayed outcomes.
5. Support fixed, global, and targeted learning gates through one common implementation.
6. Support deterministic replay, freezing of plasticity, and gate interventions.
7. Generate a report comparing multiple independent runs, not just a visually appealing demonstration.

The first good result can be negative. For example: a tuned global gate performs as well as targeted gates, or activity-only memory solves the task without synaptic changes. Those outcomes become informative only when the implementation and controls are trustworthy.

### 2.2 Things not to build first

Do not initially implement a 3D world, a realistic body, neuron growth, evolving topology, multiple competing agents, detailed spiking dynamics, a learned reward function, or a large neural network that generates the update rule. Do not add a separately trained readout that could solve the task on behalf of the recurrent network.

Do not optimize directly for gate patterns that look interesting. Optimize actual environmental performance, then analyze the gate patterns.

### 2.3 Recommended first visible demonstration

A dashboard or static diagnostic report showing:

- the current cue, committed action, and delayed outcome;
- rolling accuracy and performance around actual association changes;
- a small sample of neuron activities and eligibility traces;
- the global or targeted gate values;
- the connection changes actually applied after each outcome.

Hidden change points can appear in researcher-only plots. They must never be passed back into the agent.

---

## 3. System architecture

### 3.1 Components

```text
Observable world inputs
        |
        v
+--------------------------+
| Recurrent actor network  | ---- fixed motor populations ----> action
| persistent h, adaptation |
| inherited W0 + acquired P|
+--------------------------+
      |             ^
      | activity    | local weight changes
      v             |
+--------------------------+
| Recurrent modulator      |
| observes ordinary inputs |
| and actor activity       |
| emits learning gates g   |
+--------------------------+
                    |
                    v
         eligibility * outcome signal * gate
                    |
                    v
           acquired synaptic state P

Between complete lifetimes only:
   evolutionary search changes inherited parameters
```

This is a conceptual data-flow diagram. There is no external task-solving model and no backpropagation through an agent's lifetime.

### 3.2 Starting sizes

Use a small actor first, then the main scale:

| Component | Debug profile | Main starting profile |
|---|---:|---:|
| Actor neurons | 16 | 60 |
| Motor neurons | 2 per action | 4 per action |
| Modulator neurons | 2 | 4 |
| Cue types | 2 | 8 |
| Actor recurrent edge probability | 0.25 | 0.15 |
| Actions | 2 | 2 |

Motor neurons are part of the actor count. In the main profile, 52 actor neurons are non-motor, 8 are motor neurons, and 4 additional units form the modulator. This gives 64 continuously evolving neuron-like units in total.

### 3.3 Why separate the modulator initially

The modulator is part of the agent, but its outputs affect only plasticity in the first version. It does not send an ordinary activation signal back into the actor.

This restriction makes causal analysis cleaner. Clamping a gate does not simultaneously remove a direct motor-control pathway. Later, ordinary feedback from modulator to actor can be enabled as an explicitly different architecture.

The restriction is a research instrument, not a biological assertion that modulation and computation are cleanly separated in brains.

### 3.4 Boundary between simulator and agent

The environment owns hidden associations, hazards, noise probabilities, event schedules, correctness, and evaluation metadata. The agent owns neural activity, local traces, acquired weights, its reward baseline, motor filters, and modulation state.

Only an `Observation` object crosses from environment to agent. The agent returns only a continuously available `MotorOutput`. The environment latches an action at the defined commitment event.

The evaluator may inspect hidden truth for analysis. The fitness function and plotting code must not have a path back into a running agent.

---

## 4. Notation and time conventions

### 4.1 Indexing

- `t`: integer simulation tick.
- `n`: committed choice/outcome index within a lifetime.
- `c`: cue identity.
- `i`: presynaptic/source actor neuron.
- `j`: postsynaptic/receiving actor neuron.
- `k`: modulator neuron.
- `W[j, i]`: weight from actor neuron `i` to actor neuron `j`.

**Rows are receivers; columns are senders.** Use this convention in equations, array storage, visualizations, and tests.

### 4.2 Principal state

| Symbol | Meaning |
|---|---|
| `h[j]` | actor membrane-like continuous state |
| `r[j] = tanh(h[j])` | actor signed activity |
| `a[j]` | slow signed adaptation state |
| `W0[j,i]` | inherited actor recurrent weight |
| `P[j,i]` | acquired plastic offset |
| `W = W0 + P` | effective actor weights on existing edges |
| `B[j,d]` | inherited input-to-actor weight |
| `b[j]` | inherited actor bias |
| `E[j,i]` | local eligibility trace |
| `z[k]` | modulator recurrent state |
| `m[k] = tanh(z[k])` | modulator activity |
| `g[j]` | receiving-neuron learning gate |
| `q[0], q[1]` | continuously filtered motor-population outputs |
| `R[n]` | observed scalar reward, either 0 or 1 |
| `b_R` | running reward baseline |
| `delta[n]` | reward minus baseline at an outcome event |

The reward baseline `b_R` is unrelated to the actor bias vector `b`. Use distinct field names in code.

### 4.3 Time units

Set `dt = 1` simulation tick. Do not label ticks as milliseconds unless a later experiment deliberately establishes that interpretation.

For a time constant `tau > 0`, define:

```text
alpha(tau) = 1 - exp(-dt / tau)
lambda(tau) = exp(-dt / tau)
```

Use `-expm1(-dt / tau)` to compute `alpha` accurately for large `tau`.

Network states advance every tick, including quiet intervals and delays. Plastic offsets change only on outcome events in the main experiment. This still counts as continuous operation: the system is not reset or paused between interactions.

---

## 5. Environment and information boundary

### 5.1 Task: noisy contextual reversal

Each cue `c` has a hidden preferred action `y[c]` in `{0,1}`. At birth, choose each `y[c]` independently with probability one half.

After the agent commits action `A`, define:

```text
correct = (A == y_at_commit)
noise_flip ~ Bernoulli(epsilon[c])
R = correct XOR noise_flip
```

Interpret booleans as 0 or 1. Equivalently:

```text
P(R = 1 | correct action) = 1 - epsilon[c]
P(R = 1 | wrong action)   = epsilon[c]
```

Use `0 <= epsilon < 0.5` for the main task. An `epsilon = 0.5` condition is a useful information-free negative control, not a learnable target.

The reward distribution is sampled once for each committed choice and delivered later. Never recompute correctness against a subsequently changed mapping.

### 5.2 Hidden changes

For each cue, assign a hazard `hazard[c]`. At each presentation of cue `c`, before presenting its sensory input:

```text
if c has been presented before:
    with probability hazard[c]: y[c] = 1 - y[c]
```

The first presentation uses the birth mapping without an additional flip. Hazards are defined **per exposure of that cue**, not per global decision and not per tick.

In the main profile, half the cues have hazard zero and half have nonzero hazards. Randomize which cue IDs belong to each group at birth. Do not encode volatility in the cue's vector magnitude, position, color, or frequency.

A memoryless hazard does not guarantee enough time between flips for recovery. Use a separate isolated-change diagnostic for clean event-aligned recovery measurements; do not quietly impose a minimum dwell time on the main task.

### 5.3 Noise and volatility assignment

Training should include a factorial set of noise and hazard conditions, rather than making noisy cues always stable or always volatile.

Suggested initial training values:

```text
feedback noise epsilon:       0.00, 0.10, 0.20
nonzero hazard per exposure:  0.005, 0.020
stable hazard:               0
```

Counterbalance noise assignments across stable and volatile cues and across cue identities. Use all combinations in the ordinary training distribution. In particular, there must be noisy stable cues and relatively reliable volatile cues.

Do not pass these numeric parameters to the agent. They are only environment and evaluator parameters.

### 5.4 Cue representation

Begin with one-hot vectors of dimension `K`. During cue presentation, input dimension `c` is 1 and all other cue dimensions are 0. Outside cue presentation, all cue dimensions are 0.

This is intentionally simple. The first question is about learning regulation, not representation learning. Later, test dense overlapping cue encodings while keeping all other machinery unchanged.

A fixed random projection through `B` converts cues into distributed actor input. Avoid hand-wiring each cue to its own isolated plastic memory slot in the main network; that could pre-solve the question of spatial targeting.

### 5.5 Observable input channels

Use `D = K + 6` actor/modulator input dimensions:

| Channel | Meaning | When nonzero |
|---|---|---|
| `cue[0..K]` | one-hot cue content | cue presentation |
| `cue_present` | distinguishes absent cue from content | cue presentation |
| `go` | a public response-period signal | response period |
| `outcome_present` | an outcome is being delivered | exactly one feedback tick |
| `outcome_value` | observed reward `R` | feedback tick; may be zero |
| `previous_action_0` | most recently committed action was 0 | after a commitment |
| `previous_action_1` | most recently committed action was 1 | after a commitment |

Before the first commitment, both action channels are zero. Afterward exactly one is one until the next commitment. This is an efference-copy-like input: the agent is allowed to know what action it took.

An outcome of zero must remain distinguishable from no outcome. This is why `outcome_present` is a separate channel.

### 5.6 Inputs that are forbidden

Never expose the preferred action, correctness, `noise_flip`, cue hazard, cue noise rate, stable/volatile membership, hidden change flag, future cue sequence, remaining lifetime duration, test/training split identity, or a simulator-only event ID as a neural input.

An event ID may exist in infrastructure to prevent duplicate delivery. It is not a sensory feature. Do not expose a countdown to the reward unless it is a deliberately added experimental condition.

### 5.7 Event sequence

The first environment allows one unresolved action at a time:

```text
quiet interval
    -> cue presentation
    -> optional blank memory interval
    -> response interval, go=1
    -> commit at final response tick
    -> reward delay
    -> one feedback tick
    -> next quiet interval
```

Starting durations, in ticks:

| Phase | Debug | Main starting profile |
|---|---:|---:|
| Quiet interval | 4 | uniform integer 8..16 |
| Cue presentation | 8 | 16 |
| Blank memory interval | 0 | uniform integer 0..8 |
| Response interval | 4 | 8 |
| Commitment-to-feedback delay | 1 | uniform integer 8..24 |
| Feedback | 1 | 1 |

Each interval includes exactly the declared number of ticks. Specify this with table-driven tests rather than relying on ambiguous loop bounds.

The commitment tick is `t_commit`. The outcome is delivered at the **start** of tick `t_commit + delay`, where `delay >= 1`. No new cue begins before that feedback tick has completed.

The agent is not reset at any of these boundaries. Only the environment's phase changes.

### 5.8 Action commitment

The network produces continuous motor outputs at every tick. The environment reads them only at commitment. It stores the chosen action and the then-current target.

Actions do not affect cue selection, future hazards, or the noise schedule in the initial environment. This lets different agents face the same externally generated schedule. Their rewards may differ because their actions differ.

Use separate pseudorandom streams for cue selection, mapping changes, reward noise, timing, neural perturbations, and evolution. A different number of neural random draws must not alter the task schedule.

### 5.9 Curriculum environments

Implement these as separate named profiles, not hidden special cases:

- `stationary_clean`: no changes, no feedback noise, short delays.
- `stationary_noisy`: no changes, increasing feedback noise.
- `isolated_reversal`: one known-to-the-evaluator change, with a long clean observation window.
- `mixed_continual`: interleaved stable and volatile cues, independent noise assignments, no resets.
- `long_life`: frozen trained agent evaluated for several multiples of the training lifetime.
- `uninformative_reward`: feedback noise 0.5, used only as a negative control.

---

## 6. Neuron dynamics and action interface

### 6.1 Actor update

At the start of a neural transition, let `r_old = tanh(h_old)` and use the current effective weights after any feedback-event update specified in Section 9.

For every actor neuron `j`:

```text
alpha_h[j] = 1 - exp(-1 / tau_h[j])
alpha_a[j] = 1 - exp(-1 / tau_a[j])

drive[j] = sum_i W[j,i] * r_old[i]
           + sum_d B[j,d] * input[d]
           + actor_bias[j]
           - adaptation_strength[j] * a_old[j]

mu[j] = (1 - alpha_h[j]) * h_old[j] + alpha_h[j] * drive[j]

xi[j] ~ Normal(0, 1)
h_new[j] = mu[j] + sigma[j] * xi[j]

a_new[j] = (1 - alpha_a[j]) * a_old[j] + alpha_a[j] * r_old[j]
r_new[j] = tanh(h_new[j])
```

All right-hand sides use old arrays. Compute into new arrays and swap. Do not accidentally use a freshly updated neighbor in the same tick.

The perturbation is added **after** the leaky integration. Its standard deviation is `sigma` per tick. Moving it inside the `alpha_h` factor changes both the dynamics and the learning equation.

### 6.2 Adaptation interpretation

`a` is a slow signed moving average. Subtracting it opposes sustained activity in either direction. This is an abstract fatigue/adaptation mechanism, not a literal refractory current or a biologically calibrated firing-rate model.

Set `adaptation_strength = 0` until the basic local learner works. Then compare the same experiment with and without adaptation. Do not add it simultaneously with a new learning rule and a harder task.

### 6.3 Starting parameters

```text
actor tau_h:                 5 ticks
actor tau_a:                 100 ticks
adaptation_strength:         0 initially; 0.1 for a later ablation
actor perturbation sigma:    0.05 per tick
motor filter tau_q:          3 ticks
```

Keep `sigma > 0` wherever the stochastic score rule is active. Setting noise to zero during learning is not supported by that rule. Noise-free evaluation is a separate intervention and not the default deployment condition.

### 6.4 Motor readout

Choose fixed disjoint motor sets `M0` and `M1`, each with the same number of neurons. Do not train a separate decoder.

```text
motor_mean[a] = mean(r_new[j] for j in Ma)
q_new[a] = (1 - alpha_q) * q_old[a] + alpha_q * motor_mean[a]
```

At commitment:

```text
A = 0 if q[0] > q[1] else 1 if q[1] > q[0] else fair_coin()
```

Use a dedicated tie-breaking random stream. The continuously injected actor noise supplies ordinary exploration; do not initially add epsilon-greedy action selection or a second softmax exploration mechanism.

The action map is deterministic apart from exact ties, but the underlying neural trajectory is stochastic. That is sufficient for the score-based diagnostic in Section 7.

### 6.5 Observability checks

Before learning, verify that distinct cues produce distinguishable network activity and that both actions occur across seeds. A network locked into one saturated motor state is not a useful starting point for studying modulation.

There must be directed paths from cue-driven actor neurons to motor neurons. The main fixed recurrent mask should contain feedback cycles. These are structural sanity checks, not evidence of learning.

---

## 7. Local credit assignment

### 7.1 Why start with explicitly measured perturbations

The first rule should distinguish an exploratory fluctuation from ordinary recurrent activity. Otherwise a failed experiment can be impossible to diagnose: is the task hard, the credit signal wrong, or modulation ineffective?

Likelihood-ratio reinforcement learning provides a foundation for updates using a stochastic unit's local probability score. [R1] The author's code accompanying Miconi's recurrent-learning paper also contains a node-perturbation implementation distinct from its more biologically motivated rule. [R7]

The model below is a deliberately specified Gaussian-transition version. It is not a claim to reproduce Miconi's published equations or e-prop. Its conditional score is derived here from the transition distribution and must be tested independently.

### 7.2 Derive the per-transition score

For a single receiving neuron under the dynamics above:

```text
h_new[j] ~ Normal(mu[j], sigma[j]^2)
mu[j] = ... + alpha_h[j] * W[j,i] * r_old[i] + ...
```

Holding the previous state fixed, differentiation of this Gaussian log probability gives:

```text
S[j,i] = d log p(h_new[j] | old_state, input) / d W[j,i]
       = alpha_h[j] * r_old[i] * (h_new[j] - mu[j]) / sigma[j]^2
       = alpha_h[j] * r_old[i] * xi[j] / sigma[j]
```

This uses the presynaptic activity, receiving neuron's own perturbation, and fixed neuron parameters. It does not use other neurons' future states, the correct action, or a backward sweep through the network.

Do **not** multiply this score by `tanh'(h_new)`. It is a derivative of the sampled membrane transition's log probability, not a deterministic backpropagation derivative through the activation.

Do **not** use `xi[i]` instead of `xi[j]`. One postsynaptic perturbation is shared by all incoming edges to that neuron on that tick.

### 7.3 Persistent eligibility

For each plastic actor edge:

```text
lambda_e = exp(-1 / tau_e)
E_new[j,i] = lambda_e * E_old[j,i] + S[j,i]
```

Starting `tau_e = 64` ticks for the main profile. This is a tunable default, not a biological timescale.

Do not multiply the score by `1 - lambda_e` unless implementing and naming a different normalization. Such a factor changes the effective learning rate and must be accounted for in comparisons.

For fixed zero-mean independent score increments, longer traces can accumulate more variance. Real recurrent scores can be correlated, so measure empirical trace magnitudes rather than assuming a simple independent-noise variance formula holds exactly.

### 7.4 Reward baseline and teaching signal

On feedback event `n`, before updating the baseline:

```text
delta = R - reward_baseline
reward_baseline_new = reward_baseline + beta_R * (R - reward_baseline)
```

Initialize `reward_baseline = 0.5`. Start `beta_R = 0.02` per outcome, not per tick.

This is a running performance reference, not a learned state-value function and not a temporal-difference critic. It is intentionally weak and contains no cue-indexed task solution.

For the exact finite-horizon score diagnostic, hold the baseline constant during a rollout and set it independently of that rollout's perturbations. The continually updated baseline in the main experiment is an online heuristic; do not transfer the diagnostic's unbiasedness claim to it without further analysis.

### 7.5 Weight update

At an outcome event, read the existing trace and the gate defined by Section 8:

```text
raw_update[j,i] = eta * delta * gate[j] * E[j,i]
limited_update[j,i] = clamp(raw_update[j,i], -max_update, +max_update)
P_new[j,i] = clamp(P_old[j,i] + limited_update[j,i], -plastic_bound, +plastic_bound)
W_new[j,i] = W0[j,i] + P_new[j,i]
```

Apply updates only on edges in the plastic mask. Missing edges stay missing. Biases, sensory weights, and modulator weights are not lifetime-plastic in the first main experiment.

Starting values:

```text
eta:            0.001 per outcome
max_update:     0.01 per edge per outcome
plastic_bound:  0.5 per edge
```

Log `raw_update`, `limited_update`, and the actual change after the plastic-bound clamp. A bound-saturated system can appear to have closed learning gates when clipping is the real reason updates disappear.

Do not decay `P` by default. Automatic weight decay would add another forgetting mechanism and confound the noise-versus-change comparison. Add it only as a named condition applied consistently across baselines.

### 7.6 What is exact, and what is not

The local Gaussian score above is exact for the stated conditional transition distribution.

A stronger trajectory-gradient interpretation requires a restricted diagnostic: fixed weights throughout a finite rollout, an initial state independent of those weights, fixed nonzero noise scale, no state clipping, no eligibility decay, an appropriate constant baseline, and one update after the rollout. Sum the local transition scores over the rollout and multiply by its terminal reward minus baseline. This gives a score-function estimator for that finite-rollout expected reward.

The main experiment deliberately departs from those conditions: it uses decaying traces, weights that change over a lifetime, a changing baseline, bounds, and possibly state-dependent gates. Treat it as a **heuristic online local learning system**. Do not claim an unbiased gradient of total lifetime fitness.

Finite-difference checks of the restricted diagnostic validate the simulator and score arithmetic. They do not prove convergence or correctness of the complete gated online learning algorithm.

### 7.7 Why the trace is not cleared after a reward

In the main continuous condition, eligibility decays naturally and is never zeroed at choice boundaries or feedback events. Some trace components can therefore influence more than one outcome. That is part of this continuing approximation and can cause cross-choice interference.

Implement an explicit `event_reset_diagnostic` condition that clears traces after outcomes. It can help identify interference, but it must not be mislabeled as the no-reset main model.

Also implement a finite-rollout diagnostic that resets state and traces between independent rollouts. This is a mathematical verification tool, not the continuous research result.

### 7.8 Perturbations after commitment

With the chosen ordering, scores accumulated after an action commits but before its outcome arrives are included in the trace. These later perturbations cannot change the already committed action, so they can add variance to that outcome's update.

For the fixed-weight, constant-baseline diagnostic, their contribution has zero expectation when the reward is independent of those later perturbations. In the full online gated model, do not assume every such cancellation survives.

Keep reward delays short at first. A later diagnostic may snapshot eligibility at commitment and decay only that snapshot until feedback. Name it `commit_snapshot_credit`: it introduces an engineered action-credit boundary and is not the same rule as continuously accumulated eligibility.

### 7.9 Trace timescale trade-off

A trace increment from `D` ticks ago is multiplied by `exp(-D/tau_e)`. With `tau_e=64` and `D=16`, its retained factor is approximately 0.779. This is direct arithmetic, not a performance prediction.

Increasing `tau_e` preserves older credit but also includes more unrelated activity and previous choices. Tune `eta` jointly with `tau_e`; changing the trace timescale can change both useful memory and update scale.

### 7.10 Biological limitation to keep visible

The receiving neuron is given its own injected perturbation `xi` by the simulator. This is computationally local but an engineered credit-assignment aid. The first result should be described as a brain-inspired learning mechanism, not proof of how real neurons measure their causal contribution.

After the primary study works, replace the perturbation-access rule with a carefully implemented alternative, such as a published supralinear fluctuation rule, and rerun the same controls. Do not casually substitute pre/post co-activity and assume it estimates the same thing. [R2]

---

## 8. Internal learning gates

### 8.1 Modulator dynamics

The modulator receives current ordinary sensory input and the actor's previous activity. It has its own recurrent state:

```text
alpha_m = 1 - exp(-1 / tau_m)

mod_drive[k] = sum_j C[k,j] * r_old[j]
               + sum_l H[k,l] * tanh(z_old[l])
               + sum_d U[k,d] * input[d]
               + mod_bias[k]

z_new[k] = (1 - alpha_m) * z_old[k] + alpha_m * mod_drive[k]
m_new[k] = tanh(z_new[k])
```

Start `tau_m = 20` ticks. The first modulator is deterministic given its inputs; it receives actor variability through `r_old`.

All `C`, `H`, `U`, and modulator biases are inherited, not learned within a lifetime. Evolution may modify a restricted subset described in Section 12.

### 8.2 Three gate modes

**Fixed plasticity:**

```text
gate[j] = 1
```

The effective learning rate is `eta`.

**Global internal gate:**

```text
global_gate = sigmoid(global_bias + dot(global_projection, m))
gate[j] = global_gate for every receiver j
```

**Targeted internal gate:**

```text
gate[j] = sigmoid(gate_bias[j] + sum_k V[j,k] * m[k])
```

Targeting is postsynaptic: all plastic edges arriving at receiver `j` share `gate[j]`. It is not a separate independently generated gate for every synapse. This keeps the mechanism and parameter count manageable.

Use a numerically stable sigmoid. Gates lie in `[0,1]` and regulate magnitude, not sign. The reward signal and eligibility determine the sign of each update.

### 8.3 Primary timing: pre-outcome gates

For the main experiment, sample gates from the internal state **before the current reward is observed**. At the start of a feedback tick, read `gate_old`; apply the weight update; then advance the network with the feedback input.

This means the gate can use prior outcomes, the current cue's remembered context, the agent's own activity, and accumulated history. It cannot immediately react to the sign of the reward that it is about to gate.

This is a deliberate restriction. It makes the initial mechanism easier to audit and reduces one direct reward-dependent gating confound. It does not by itself restore an unbiased-gradient guarantee.

The modulator can react to today's outcome when setting gates for subsequent experience. The agent can therefore learn a history-dependent learning policy even though the current event uses a pre-outcome gate.

### 8.4 Optional outcome-responsive timing

A later condition may let the circuit process a reward before selecting the gate for that reward. Implement it explicitly:

1. At outcome arrival, store the pre-feedback eligibility snapshot, reward, and baseline value.
2. Deliver the outcome to the network for a fixed number of processing ticks.
3. Read the resulting gate.
4. Apply exactly one update using the stored eligibility and stored reward-minus-baseline.
5. Update the baseline once.

Do not include scores from processing the reward in that stored eligibility. Avoid starting a new choice before this update completes in the first version.

This is `post_feedback_gate`, a different algorithm with an engineered short-lived credit buffer. Reward-dependent gates can systematically change update direction in expectation even when nonnegative. Evaluate them empirically and do not present them as ordinary unbiased policy gradients.

### 8.5 Gate initialization

For a fully specified initial controller, let `N` be actor count, `M` modulator count, and `D` input dimension. Initialize `C[k,j]` from `Normal(0, 0.1^2/N)`, `H[k,l]` from `Normal(0, 0.5^2/M)`, and `U[k,d]` from `Normal(0, 0.3^2)`. These expressions specify variance, not standard deviation. Set modulator biases to zero. The corresponding standard deviations are `0.1/sqrt(N)`, `0.5/sqrt(M)`, and `0.3`.

Draw these once per outer initialization and pair them across conditions where dimensions match. For searched parameters with the bounded genotype transform, reject and redraw an initialization outside the decoded weight bounds, then encode it with `atanh(weight/weight_limit)`. Do not confuse initial decoded weights with genotype values.

Initialize gate projection weights near zero and gate biases to zero, producing gates near 0.5. Initialize fixed plasticity with a separately tuned `eta`; do not accidentally give it twice the effective update magnitude and call the comparison fair.

Gates that immediately saturate near zero can stop learning before selection has a useful signal. Track their distributions and begin with modest projection scales.

### 8.6 What must not be supplied to the gates

The modulator must never receive the hidden change flag, true correctness, stable/volatile label, noise bit, or cue-specific true reward probability. Do not hard-code a change detector inside the targeted network while presenting its behavior as an evolved discovery.

A hand-designed adaptive learning-rate baseline is useful, but it belongs in a separately named comparator.

---

## 9. Exact tick and event ordering

### 9.1 Authoritative order for the primary model

The following order overrides informal descriptions elsewhere:

```text
for each tick t in one lifetime:

    1. Environment creates the observable input for tick t.
       It may attach one pending feedback event to this input.
       Hidden annotations are sent only to the logger/evaluator.

    2. If feedback is present:
         a. Check event identity has not already been consumed.
         b. Read pre-feedback E, gates, P, and reward_baseline.
         c. Compute delta = observed_reward - reward_baseline.
         d. Compute and apply bounded plastic updates.
         e. Update reward_baseline once.
         f. Mark feedback event consumed.
       Do not clear h, a, z, q, E, or P.

    3. Save r_old, h_old, a_old, z_old, and q_old.

    4. Compute actor mu from old activity and current weights.
       Draw one independent Gaussian perturbation per actor neuron.
       Compute h_new, a_new, and r_new.

    5. Compute modulator z_new using old actor/modulator activity
       and the current observable input.

    6. Update each live eligibility trace:
         E_new = lambda_e * E_old + alpha_h * r_old * xi / sigma.
       These new scores cannot be used for feedback already consumed
       at the start of this tick.

    7. Update filtered motor outputs using r_new.

    8. Compute gates from z_new for use on future feedback ticks.

    9. If this is the final response tick:
         a. Commit the action from the new motor outputs.
         b. Environment stores action and mapping-at-commit.
         c. Environment schedules exactly one future reward event.
         d. Previous-action sensory latch changes starting next tick.

   10. Write requested logs, then advance the environment clock.
```

### 9.2 Why feedback comes before the current neural transition

A feedback-triggered update must use only traces that existed before processing that feedback. Otherwise the system can create correlations from the sensory reaction to an outcome and mistakenly use them to explain the action that produced it.

This ordering also defines precisely which gate is considered pre-outcome.

### 9.3 Minimal pseudocode API

```text
observation, hidden_annotation = environment.observe()

if observation.feedback is present:
    agent.apply_feedback_once(observation.feedback)

motor = agent.advance_neural_tick(observation.features)

if environment.commitment_due():
    action = choose_from_motor(motor, tie_rng)
    environment.commit(action)

evaluator.record(environment, agent, hidden_annotation)
environment.finish_tick()
```

`agent.advance_neural_tick` must not independently apply feedback again. A second update path hidden inside that method would double every reward event.

### 9.4 Illustrative timeline

Suppose an action commits at tick 20 and the delay is 3:

```text
tick 20 end: action committed; reward due at tick 23
tick 21:     ordinary transition and eligibility update
tick 22:     ordinary transition and eligibility update
tick 23 start:
             apply one reward update using E through tick 22
             and gates through tick 22
tick 23 transition:
             process reward as sensory input; create future trace/gates
```

Write an exact test for this timeline before running an experiment.

---

## 10. Initialization, bounds, and resets

### 10.1 Recurrent mask

Sample a directed Bernoulli mask with the configured edge probability. Set the diagonal to zero initially. Store the mask permanently for that initialization seed.

Check cue-to-motor reachability and the presence of recurrent cycles. Reject structurally unusable masks with an explicit logged reason. Do not select masks based on test performance.

Use paired masks across gate conditions within an outer seed. Across independent outer seeds, vary the mask so a result is not a claim about one lucky reservoir.

### 10.2 Inherited actor weights

For each receiving row with in-degree `d_j > 0`:

```text
W0[j,i] ~ Normal(0, recurrent_gain^2 / d_j) on existing edges
```

Start `recurrent_gain = 0.8`. Set missing edges to zero.

For input weights, use a modest fixed random projection, initially `Normal(0, input_scale^2)` with `input_scale = 0.3`. Because few input channels are active simultaneously, inspect the actual input-current distribution rather than applying an unexplained dimension-normalization formula.

Use zero actor biases initially. Symmetry is already broken by random weights and perturbations.

A gain or spectral-radius heuristic is not a proof of stability for a nonlinear noisy plastic network. Observe bounded state statistics, saturation, and numerical finiteness under the actual lifetime protocol.

### 10.3 Plastic mask

The simplest full-network choice is that every existing actor recurrent edge is plastic. Early debugging may restrict plasticity to edges entering motor neurons.

Do not compare a motor-only fixed baseline against a fully plastic targeted system and attribute all differences to modulation. Match plastic masks for the primary comparison.

### 10.4 Birth reset

At the start of each independent lifetime:

```text
h = 0; a = 0; z = 0; q = 0
P = 0; E = 0
reward_baseline = 0.5
previous_action channels = 0
pending reward = none
last_consumed_feedback_id = none
```

Load inherited weights and parameters from the genome. Initialize random streams from the lifetime's deterministic seed tuple.

A fixed quiet warmup of 32 ticks may be used before the first cue. It is part of the lifetime simulation, and traces evolve during it. Record whether warmup is enabled. Do not secretly reset traces after warmup in the continuous condition.

### 10.5 No within-lifetime resets

Do not clear state when a cue changes, a reward arrives, a hidden association changes, a plotting window ends, or a logging file rotates.

Diagnostic resets are allowed only under explicitly named profiles. Checkpoint/restart must preserve the complete state rather than approximating a new birth.

### 10.6 Bounds and failure handling

Use `f64` for the first reference implementation. Reject or flag any nonfinite state, weight, trace, gate, baseline, or fitness.

Do not silently clip membrane state `h`; doing so changes the stochastic transition distribution underlying the score derivation. A finite-state watchdog may stop and mark a run as failed. Choose its threshold conservatively and log it.

Clip per-event plastic updates and plastic offsets as specified. Record the fraction clipped. If clipping is frequent, the bounds may be driving the result rather than merely protecting it.

A failed candidate receives a predetermined finite worst fitness and a failure code. Never discard failures from aggregate results merely because their plots are inconvenient.

### 10.7 Full pause/resume

A resumable agent checkpoint includes neural state, adaptation, modulator state, motor filters, plastic offsets, eligibility, baseline, feedback-consumption bookkeeping, environment phase and pending reward, all RNG states/counters, inherited parameters, and configuration hash.

A genome file alone is not a lifetime checkpoint.

---
## 11. Inherited parameters and lifetime memory

### 11.1 Genome versus phenotype

The **genome** is everything restored at birth. The **lifetime state** is everything accumulated through interaction.

| Inherited | Acquired or transient within a lifetime |
|---|---|
| Actor mask and plastic mask | Actor and adaptation states |
| Initial actor weights `W0` | Acquired offsets `P` |
| Input projection `B` and actor biases | Eligibility traces `E` |
| Modulator connectivity and gate projections | Modulator state and current gates |
| Fixed neuron time constants | Motor filters |
| Learning-rate and trace parameters | Running reward baseline |
| Fixed motor-neuron assignment | Last action and feedback bookkeeping |

A candidate evaluation always starts from a fresh birth. Do not accidentally carry the trained `P` from one evaluation lifetime into the next.

### 11.2 No inheritance of acquired weights initially

Evolution changes inherited parameters based on lifetime fitness. It does not copy learned offsets `P` into the next generation's `W0`.

This is a deliberate non-Lamarckian protocol. Inheritance of acquired weights could be interesting later, but it changes what the project demonstrates and must be a separate condition.

### 11.3 Why fixed weights do not imply no adaptation

A recurrent network can change its behavior based on history while all weights remain fixed. Information can be carried in its activity state. Meta-reinforcement-learning work explicitly studies this possibility. [R6]

Therefore distinguish:

- **Activity-mediated adaptation:** experience changes `h`, `a`, `z`, or other transient state.
- **Synaptic adaptation:** experience changes `P` and those changes affect later behavior.
- **Inherited specialization:** evolution has installed a behavior or learning strategy in the initial parameters.

A learning curve alone does not identify which of these occurred.

### 11.4 Two levels of claim

A gate-only evolutionary search on a fixed actor can establish that modulation helps **that family of locally plastic actors**. It cannot establish superiority over the best activity-only recurrent agent, whose initial circuitry was never optimized.

For a broader claim, use the separately optimized activity-only comparator and matched joint-search conditions in Section 13. Keep these two levels of evidence separate in the report.

---

## 12. Evolutionary search

### 12.1 Do not evolve before the base learner works

First tune an always-on local learner on a clean stationary task. Then introduce noisy feedback and reversals. Only after there is a real adaptation/noise trade-off should evolution search for gates.

A nonlearning actor can make an effective gate circuit look useless. A gate circuit that simply suppresses damaging updates can look useful even when the underlying rule never learned anything. Both failure modes require a known-working baseline.

### 12.2 First search: gate-only discovery

Keep actor topology, `W0`, `B`, actor bias, motor mapping, noise amplitude, and plastic mask fixed within an outer replicate. Search only the modulator and gate parameters.

For the main targeted model, one practical genome includes:

```text
C:                  4 x 60 actor-to-modulator weights
H:                  4 x 4 modulator recurrent weights
mod_bias:           4
V:                  60 x 4 gate projection weights
gate_bias:          60
```

This is 560 scalar parameters. Keep `U`, the sensory-to-modulator projection, fixed and paired across conditions initially. Keep `tau_m`, `tau_e`, and `eta` fixed during this first search, after tuning them on development tasks.

The global-gate version has the same `C`, `H`, and modulator biases but one output projection and one output bias, totaling 265 searched parameters under this parameterization.

Different dimensions are a real architectural difference. Report them. Equal simulation budgets do not make search difficulty identical.

### 12.3 Smaller discovery search

Before searching all 560 parameters, use a reduced profile:

- Freeze `C`, `H`, and `U` at small random values.
- Search only `V` and gate biases.
- Optionally use 16 actor neurons and 2 modulator neurons.

This asks whether a small projection of an existing recurrent history representation can usefully gate learning. It does not demonstrate that evolution invented the entire internal representation.

If this fails, do not assume the hypothesis is false. Check whether the frozen modulator represents relevant history at all, then decide whether to open the recurrent modulator parameters to search.

### 12.4 Parameter encoding

Maintain unconstrained genotype values `theta`. Decode bounded weights using:

```text
weight = weight_limit * tanh(theta)
```

Start `weight_limit = 2` for searched modulator and gate weights. Treat that as a development parameter. Very large decoded weights often saturate modulators or gates.

For any later searched positive scalar, use a bounded log-space transform:

```text
log_x = log(x_min) + sigmoid(theta) * (log(x_max) - log(x_min))
x = exp(log_x)
```

Suggested later search ranges:

```text
eta:       1e-5 .. 1e-2
tau_e:     8 .. 256 ticks
tau_m:     2 .. 128 ticks
```

Do not evolve `sigma` initially. Changing noise amplitude changes exploration and the numerical scale of the eligibility score, complicating interpretation.

### 12.5 Simple evolutionary algorithm to implement first

Use a deliberately simple elitist mutation-and-selection search rather than implementing a sophisticated optimizer before the simulator is trustworthy.

Starting settings:

```text
population_size = 32
elite_count = 8
mutation_std = 0.10 in genotype space
```

Algorithm:

```text
initialize a population around the near-half-open gate solution

for each generation:
    generate a fresh, stratified training batch of lifetime seeds

    evaluate every candidate on exactly that same batch
    compute fitness from complete lifetimes
    sort descending by fitness, with deterministic tie handling

    select the top elite_count candidates
    preserve those genotypes as the first entries of the next population

    fill remaining entries:
        choose an elite parent uniformly
        child_theta = parent_theta + mutation_std * standard_normal_vector

    periodically evaluate the current best genotype on validation lifetimes
    checkpoint population, scores, RNG states, and validation record
```

The preserved elites are **re-evaluated on the next generation's new batch**. Do not compare a child's score on new lifetimes against a cached parent's score on easier old lifetimes.

Treat mutation scale as a prespecified configuration, or compare a small set of scales on development data. Do not keep manually changing it in response to final-test performance.

This algorithm is a starting engineering choice. More capable evolutionary strategies can be substituted later using the same evaluation contract. Evolutionary searches for recurrent plasticity rules are established, but their success and difficulty depend on the search space and objective. [R5]

### 12.6 Lifetime batches and fitness

Use stratified batches so each candidate experiences a comparable mixture of noise levels, volatile hazards, cue identities, and initial mappings. Candidate evaluation must include fresh births.

Basic fitness:

```text
fitness = total_observed_reward / total_delivered_outcomes
```

Keep the number of completed outcomes equal across candidate lifetimes. End after a configured number of feedback events, not an arbitrary tick that leaves different numbers of rewards unresolved.

Count rewards from acquisition onward; do not discard early poor performance during fitness calculation. A separate descriptive analysis may plot late-life behavior.

The initial fitness contains no gate-shape bonus and no direct access to hidden correctness. If a numerical-health penalty is introduced, define it before comparison and use it for every condition. Report task reward separately from the penalized objective.

A system cannot improve fitness by producing its own reward signal. Fitness is computed from environment-delivered outcomes only.

### 12.7 Common random numbers

Within a candidate batch, share environment schedules across candidates. Pair initial mappings, cue sequences, hidden changes, feedback-noise draws, and timing.

For reward, sharing a noise draw does not mean forcing the same observed reward. Compute each candidate's reward from its own committed action and the shared noise bit.

Neural perturbation schedules may also be paired by `(lifetime_seed, tick, neuron_id)`. This reduces some comparison noise but does not make diverging trajectories identical.

### 12.8 Train, validation, and test

Create seed namespaces before search:

```text
development: implementation checks and hyperparameter selection
training:    evolutionary fitness batches
validation:  checkpoint/model selection
test:        final evaluation only
```

Record a manifest of the generation scheme and a hash of each finalized suite. Prevent seed overlap across namespaces.

Validation may select a generation or model checkpoint. Once the final test is inspected, further tuning creates a new exploratory cycle; do not keep treating the same test as untouched evidence.

### 12.9 Stopping and candidate selection

Use a fixed candidate-lifetime or total-tick budget. Report generations as a convenience, not the sole measure of compute.

Select the final genotype by a prespecified validation metric. Keep the evolutionary population's last candidate, highest training-fitness candidate, and best validation candidate distinct in the saved artifacts.

Do not describe a generation that happened to produce an attractive plot as the representative result.

### 12.10 Expanded search after the gate-only study

A later matched search may evolve initial actor weights and input pathways as well. Use the same actor parameterization and opportunity for optimization in every primary condition.

This is required for a strong comparison with activity-only adaptation. It is also a larger and harder search. Profile it, reduce dimensions where necessary, and report inability to optimize a comparator as a limitation rather than proof that its mechanism cannot work.

---

## 13. Baselines and fair comparisons

### 13.1 Required comparison ladder

| ID | Model | What it tests |
|---|---|---|
| B0 | Random action | Environment and scoring sanity |
| B1 | Always action 0 / always action 1 | Mapping balance and leakage checks |
| B2 | Observable-cue tabular learner | Whether a simple learner solves the task |
| B3 | Actor with plasticity disabled | Negative control for the same inherited actor |
| B4 | Always-on local plasticity, tuned fixed `eta` | Whether modulation is needed |
| B5 | Same local rule with evolved global gate | Benefit from temporal regulation |
| B6 | Same local rule with evolved targeted gates | Additional benefit from spatial regulation |
| B7 | Separately optimized activity-only recurrent agent | Stronger alternative explanation |
| O1 | Hidden-state oracle | Scoring upper reference, not a fair learner |
| O2 | Known-hazard/noise belief learner | Privileged model-based reference |

B3 is not a substitute for B7. Freezing weights in a trained plastic system is useful for mechanism analysis but does not produce a competitively trained nonplastic agent.

### 13.2 Simple tabular learner

The baseline may remember the last observed cue until its delayed reward arrives. This memory is built from observable input, not the hidden environment ID.

A straightforward version stores `Q[c, action]`, initialized to 0.5:

```text
choose argmax_a Q[c,a], with epsilon-greedy exploration
on outcome:
    Q[c, chosen_action] += tabular_lr * (R - Q[c, chosen_action])
```

Tune its learning rate and exploration rate on development or validation data. Keep both constant in the first version. Add an adaptive-rate baseline later if the recurrent mechanism is competitive.

The tabular model uses an explicit symbolic cue memory. It is an interpretable task baseline, not a neuron-level architectural match.

### 13.3 Hidden-state oracle

The oracle chooses the current preferred action at every commitment. It should achieve latent correctness 1 and expected observed reward `1 - epsilon[c]` on cue `c`.

If an ordinary agent consistently exceeds that expected reward by an amount incompatible with sampling noise, inspect scoring, noise generation, seed leakage, or outcome timing.

Observed reward in one finite run can exceed the expectation by chance. Do not use the expectation as a hard per-run upper bound.

### 13.4 Known-parameter belief reference

This comparator knows true per-cue hazard and noise rates, so it is privileged. It does not know the current hidden mapping. Use it as a reference for what the observable history could support under the assumed environment model.

Maintain `p[c] = P(y[c] = 1 | history)`, initialized to 0.5. Before a repeat exposure, apply the hidden-switch model:

```text
p_prior = hazard + (1 - 2*hazard) * p[c]
```

Choose action 1 when `p_prior > 0.5`, action 0 when lower, and break ties fairly.

After reward, form the noisy observation of the preferred action:

```text
observed_label = chosen_action if R == 1 else 1 - chosen_action

L1 = 1 - epsilon if observed_label == 1 else epsilon
L0 = 1 - epsilon if observed_label == 0 else epsilon

p[c] = p_prior * L1 / (p_prior * L1 + (1 - p_prior) * L0)
```

Use stable probability or log-odds arithmetic. Handle exact zero-noise cases explicitly; an impossible observation under a dogmatic probability should trigger an implementation check, not a hidden division by zero.

This simple filter is derived from this document's binary flip model. Do not reuse it unchanged for a differently defined noise or reversal process.

### 13.5 Search fairness

For B4-B6, match actor topology, initial actor weights within seed, motor assignment, plastic mask, sensory inputs, noise process, lifetime duration, and available training distributions.

Tune fixed `eta` jointly with trace timescale on a declared development budget. Do not give the fixed baseline one arbitrary learning rate while searching thousands of gate circuits.

Report separately:

- number of searched parameters;
- candidate-lifetime evaluations and total simulated ticks;
- validation evaluations;
- hyperparameter-selection effort;
- network size and persistent state size.

A global-gate network has fewer outputs than a targeted network. That can be a legitimate architectural comparison, but it is not automatically a pure test of locality. Add the interventions in Section 15 to support a mechanism claim.

### 13.6 Strong activity-only comparator

For B7, allow inherited actor parameters to be optimized to use recent cue, action, and reward history. Its `P` is fixed at zero for all lifetimes. The architecture must be capable of ordinary recurrent memory.

Two useful versions are:

1. Same actor neuron count as the plastic model.
2. Same total neuron count, with otherwise unused modulator units available as ordinary recurrent units.

Neither exactly matches the plastic model's much larger per-synapse memory budget. Report both neuron count and dynamic-state count rather than claiming perfect capacity matching.

### 13.7 Readout-only learning diagnostic

A system might solve the task by changing only weights entering motor neurons. Run a motor-afferent-only plastic condition with the same learning rule.

If it matches the fully recurrent plastic model, the first task has not established a need for plastic internal recurrent computation. That is a useful boundary on the result, not a reason to hide the comparator.

---

## 14. Evaluation and statistical analysis

### 14.1 Evaluation freezes evolution, not necessarily learning

During final evaluation, the genome is frozen. Lifetime plasticity remains active for models whose learning behavior is being tested. Every independent lifetime begins at birth with `P = 0`.

A separate frozen-plasticity condition tests retention or the role of online updates. Do not confuse it with ordinary generalization testing.

### 14.2 Primary performance metrics

**Observed reward rate:** mean `R` over delivered outcomes. This matches the basic fitness objective.

**Latent accuracy:** fraction of committed actions matching the hidden mapping at commitment. Use only in analysis; it separates decision quality from stochastic feedback.

**Expected regret:** for this environment, each wrong choice has expected regret `1 - 2*epsilon[c]`, and each correct choice has zero:

```text
regret[n] = (1 - 2*epsilon[c_n]) * indicator(A_n != y_n)
```

This follows directly from the two reward probabilities. It is not based on an observed reward difference and therefore has less noise from individual feedback flips.

Report per-cue and stable/volatile subgroup metrics, but do not let the agent access those subgroup labels.

### 14.3 Acquisition

For stationary tasks, plot performance by exposure number of each cue. A rare cue's tenth exposure should be compared with another cue's tenth exposure, not with its tenth global decision.

Summarize early and late exposure windows chosen before evaluation. Do not define the late window after seeing which region makes a method look best.

### 14.4 Recovery after changes

The cleanest initial recovery measurement uses `isolated_reversal`:

1. Allow acquisition of all cues.
2. Flip one cue at a randomized evaluator-only change point.
3. Prevent further flips during a declared measurement window.
4. Track correctness on the next `K` exposures of the changed cue.
5. Continue interleaving unchanged cues to measure collateral effects.

Suggested `K = 20` cue exposures. Report the number of errors in that window as a robust primary recovery measure.

A secondary metric can be the first exposure at which a rolling 10-exposure accuracy reaches 0.8. This threshold is an engineering definition, not a biological measure. If it is never reached, mark the event censored; do not replace it with zero or drop it.

For `mixed_continual`, exclude or explicitly censor overlapping reversals for this analysis. Report how many events remain. Aggregate lifetime performance must still include the full stream.

### 14.5 Stable-memory interference

Measure performance on unchanged cues around the reversal of another cue. Two approaches answer different questions:

- In the ordinary stream, compare unchanged-cue accuracy before and after a change, accounting for its normal variability.
- In paired branch experiments, compare the same unchanged-cue probes after a local change versus a matched no-change continuation.

The second is stronger for causal attribution. Use the same observable cue sequence and random schedules in both branches, with only the designated hidden change differing.

### 14.6 Misleading-outcome damage

A small update is not automatically a good update. Use both behavioral and mechanistic measures:

```text
actual_update_L1 = sum_edges abs(P_after - P_before)
actual_update_L2 = sqrt(sum_edges (P_after - P_before)^2)
```

At selected evaluator-labeled misleading outcomes, branch from the pre-outcome checkpoint. In one branch apply the ordinary update; in the other suppress that one update while still delivering the same feedback as sensory input.

Probe future behavior with subsequent plasticity frozen for a short declared window. This isolates the behavioral effect of that event's synaptic update more cleanly than comparing gate values alone.

Do not classify every nonzero update after misleading feedback as an error. In a partially observed system, updating on ambiguous evidence can be rational.

### 14.7 Gate and numerical diagnostics

Record gate mean, standard deviation, near-zero/near-one fractions, between-neuron diversity, temporal autocorrelation, eligibility norms, update norms, plastic-bound occupancy, membrane/activity statistics, and motor margins.

A relation between gates and evaluator-labeled change points is descriptive. It is not sufficient evidence that the circuit detects changes or causally protects memories.

### 14.8 Generalization suites

Use at least the following:

| Suite | Purpose |
|---|---|
| Fresh seeds under training distribution | Ordinary generalization |
| New initial mappings and cue-role permutations | Reject memorized identity rules |
| Noise 0.15 and hazard 0.01 | Interpolation between proposed training values |
| Noise 0.30 and/or hazard 0.04 | Explicitly harder extrapolation |
| New combinations of per-cue noise and hazard assignments | Reject association between specific cue IDs and dynamics |
| Longer reward delays | Test temporal-credit limits |
| Longer uninterrupted lifetimes | Reveal drift and accumulated interference |

Extrapolation failure is not automatically failure of the in-distribution hypothesis. Report each suite separately.

Do not change the number of one-hot cue input dimensions for a frozen fixed-input-size network. To test more cues, design a fixed-dimensional cue encoding first and treat that as a distinct representation experiment.

### 14.9 Independent replicates

The primary independent unit for evolved systems is the outer search replicate, including its initialization/topology seed. Many lifetimes from one selected genome are not many independent evolutionary discoveries.

Suggested progression:

```text
smoke test:           1 outer seed, a few lifetimes
exploratory study:    3-5 outer seeds
confirmation study:   initially plan 10 or more outer seeds,
                      then assess precision and feasibility
```

Ten is not a guarantee of adequate statistical power. Use pilot between-seed variability and a minimum effect of interest to plan the final comparison.

### 14.10 Confidence intervals and paired comparisons

For matched gate conditions, pair the outer initialization seeds and final environment suites. First compute a condition difference within each outer seed by averaging that seed's evaluation lifetimes. Then summarize differences across outer seeds.

Use an outer-seed bootstrap or an appropriate paired analysis. A hierarchical bootstrap may additionally resample lifetimes within outer seeds. Do not bootstrap individual ticks or individual rewards as though they were independent experimental replicates.

Report effect sizes and intervals, not only a p-value. Show every outer seed's result when feasible.

### 14.11 Preregister a small primary result set

Before the final run, specify:

```text
Primary comparison: targeted versus independently evolved global gate
Primary outcome:    expected regret in mixed_continual
Secondary outcomes: isolated-change recovery errors and unchanged-cue damage
Mechanism tests:    frozen gates, spatial reassignment, update-magnitude control
Generalization:     fresh seeds and one declared held-out condition
```

This is a suggested preregistration, not a required claim. Adjust it during development, then freeze it before the final evaluation. Label further findings exploratory.

### 14.12 Required final figures

Produce at least these five figures or equivalent tables:

1. Acquisition and continuous-lifetime performance for all primary conditions.
2. Changed-cue and unchanged-cue behavior around isolated reversals.
3. Performance across the noise-by-hazard grid.
4. Causal gate-intervention effects across outer seeds.
5. A mechanism trace showing activity, eligibility, gates, actual updates, and behavior for representative and failure cases.

Avoid selecting only the most striking lifetime. State how representative examples were chosen.

---

## 15. Causal interventions

### 15.1 General intervention protocol

Save a complete checkpoint at a predefined event. Clone it into control and intervention branches. Pair their future exogenous random schedules. Change exactly the intended mechanism, then compare behavior.

Trajectories will diverge after an intervention. Do not force equal neural states or equal actions after that point; doing so would suppress the effect being measured.

Use both acute interventions on a trained individual and separately trained restricted models. An acute intervention asks whether a trained system currently depends on a mechanism. A restricted model asks whether another system could solve the task without it.

### 15.2 Freeze future plasticity

Set the application of `P` updates to zero while preserving current `P`, activity, modulation, and sensory feedback. Eligibility may continue evolving, but never unfreeze accumulated traces without defining how they are handled.

Compare retention on stable associations and adaptation to new changes. This tests reliance on further synaptic updates, not the entire history of plasticity.

### 15.3 Erase acquired weights

Replace `P` with zero in a branch while retaining other state. Compare with an exact-copy control and a norm-matched random weight perturbation control.

An immediate behavioral disruption may reflect a general dynamical shock rather than memory content alone. Use a declared short settling/probe protocol and report the immediate effect separately.

A complementary transfer test starts a fresh dynamic state with the learned `P` transplanted into the same inherited network. Improved stationary performance after settling provides different evidence that useful information is in `P`.

### 15.4 Reset transient state

Reset `h`, `a`, `z`, and `q` while preserving `P`. For a memory-location test, also clear `E` and freeze future plasticity during the probe. Otherwise old eligibility could interact with the reset and create a new confound.

Keep environmental phase and visible-action history consistent. Prefer performing this at a quiet interval rather than halfway through a cue or reward delivery.

Do not infer a clean memory partition from one destructive reset. Use the complementary weight-reset and weight-transfer tests.

### 15.5 Constant gates

Replace generated gates with constants estimated on validation lifetimes:

- one global mean gate;
- each neuron's own mean gate.

The first removes both targeting and timing. The second preserves average spatial bias while removing temporal variation.

Retune a constant learning-rate comparator on development data. Otherwise a gate-removal failure may simply mean the effective learning rate changed.

### 15.6 Spatial reassignment

At a checkpoint, choose a fixed random permutation of receiver IDs and route each gate to the permuted receiver. Keep the gate values, modulator dynamics, and actor topology otherwise unchanged.

Repeat over several permutations. This preserves each tick's gate-value distribution but changes which synapses are regulated.

The intervention can disrupt a co-adapted system even when a separately evolved different targeting would work. Its interpretation is necessity of the learned assignment in that trained individual, not universal superiority of one spatial layout.

### 15.7 Temporal displacement

Two useful tests:

- Replay a recorded gate sequence with a fixed circular event shift.
- Replace current gates with an online delayed copy using a fixed event delay.

Recorded replay is an offline mechanistic intervention, not a deployable controller. It breaks the normal relationship between current state and gate values. A frozen no-shift replay control is needed to separate open-loop replay effects from mistiming.

Prefer event-indexed sequences when feedback timing varies. Define how to align or interpolate any tick-indexed sequence. Never inspect test outcomes to choose the most damaging shift.

### 15.8 Mean-gate control

Replace targeted gates by their mean across receivers at each event:

```text
g_global = mean_j gate[j]
```

This preserves mean gate magnitude, but not necessarily total update magnitude: receivers can have different eligibility norms. Therefore it is not sufficient by itself to show a targeting benefit independent of update size.

### 15.9 Raw-update-magnitude-matched global control

For an acute diagnostic, compute:

```text
b[j,i] = abs(eta * delta * E[j,i])
denominator = sum_plastic_edges b[j,i]

if denominator > 0:
    g_global = sum_plastic_edges b[j,i] * gate[j] / denominator
else:
    g_global = 0
```

Applying this scalar to all eligible edges matches the targeted update's **unclipped L1 magnitude** at that identical branch state.

It does not guarantee equal actual updates after per-edge clipping or plastic-bound saturation. Report raw and actual norms; restrict the cleanest analysis to events where neither branch clips.

This comparator reads all eligibilities globally. It is an evaluator-side analytical control, not a biologically local mechanism and not a fair ordinary agent.

### 15.10 Suppress one misleading-outcome update

Deliver the same reward input to both branches, but skip the synaptic update in one branch. Use short no-learning probes to assess whether the ordinary update actually harmed stable behavior.

This avoids confusing the sensory effect of bad feedback with the effect of modifying connections.

### 15.11 What evidence would support the central mechanism

A coherent positive result would combine improved performance against tuned controls, actual use of lifetime plasticity, deterioration under mistimed or reassigned gates, and persistence of an advantage after accounting for update magnitude.

No one intervention is decisive by itself. Conversely, a failed intervention can identify a simpler explanation, such as static spatial scaling or a globally reduced learning rate.

---
## 16. Implementation milestones

Each milestone should leave a small runnable command, an automated test set, and saved example output. Do not rely on an interactive notebook as the only executable record.

### M0 - Freeze contracts and build the environment

**Build:** configuration parsing, seed derivation, the phase state machine, observable inputs, pending rewards, and researcher-only annotations.

**Implement first:** `stationary_clean`, a random agent, constant-action agents, and the hidden-state oracle. Neural code is unnecessary at this stage.

**Run:** several small deterministic schedules with fixed mappings and fixed noise bits. Then run long randomized schedules to test frequencies and reward accounting.

**Required checks:** one reward per commitment, no feedback before commitment, correct handling of zero reward, correct mapping-at-commit semantics, no observations containing hidden fields, reproducible schedules independent of the agent's neural RNG.

**Exit:** all deterministic environment tests pass; oracle latent accuracy is exactly 1; random-action accuracy is statistically consistent with chance across randomized mappings; the number of committed actions equals delivered outcomes when a lifetime completes.

**Do not add yet:** evolution, recurrent neurons, or a renderer.

### M1 - Build a continuous actor with no learning

**Build:** the recurrent mask, inherited weights, double-buffered actor state, noise generator, optional adaptation state, motor filters, and motor commitment rule.

**Run:** fixed inputs, alternating cues, and long quiet periods. Log a small number of units and motor outputs.

**Required checks:** source/receiver convention, simultaneous updates, noise mean/variance, motor-filter recurrence, no state resets at environmental boundaries, and exact pause/resume on the reference platform.

**Exit:** finite states over a long smoke run, both actions reachable across initializations, distinguishable cue responses, and a checkpoint that reproduces the uninterrupted continuation.

**Interpretation:** this is a dynamical system demonstration, not learning.

### M2 - Verify the stochastic score independently

**Build:** the conditional score function and a finite-rollout accumulator with no decay.

**Run:** the closed-form one-neuron test and conditional log-probability derivative test in Section 17. Keep the baseline fixed and all weights frozen during each rollout.

**Required checks:** the receiving neuron's noise is used; the `alpha_h` factor appears once; the score divides by the actual noise standard deviation; there is no extra activation derivative.

**Exit:** deterministic derivative tests pass, and Monte Carlo estimates agree with the analytical derivative within a prespecified uncertainty tolerance.

**Interpretation:** score arithmetic works under the diagnostic assumptions. This says nothing yet about continual learning performance.

### M3 - Make an ungated local learner learn a clean task

**Build:** plastic offsets and delayed terminal updates. Begin with a deliberately episodic diagnostic: reset state and traces at rollout boundaries, use trace decay 1, and update only after a terminal reward. This is not the main continuous condition.

**Task:** two cue types, randomly assigned preferred actions, reliable reward, no hidden changes, no blank memory interval, short reward delay. Start with motor-afferent plasticity, then all recurrent edges.

**Tune only on development seeds:** `eta`, input scale, recurrent gain, and noise amplitude. Use a small explicit grid rather than unconstrained manual changes.

**Suggested development target:** substantially above-chance acquisition across several seeds; for example, median accuracy above 0.8 in the final 200 choices of a 2,000-choice clean run. This is a debugging target, not an asserted attainable benchmark.

**Exit:** learning improves over matched no-update and shuffled-reward controls; numerical checks remain valid; several seeds work rather than one lucky trajectory.

**If it fails:** reduce to a single noisy motor unit with a constant input and known preferred action. Inspect update sign before increasing network size or task complexity.

### M4 - Remove artificial trial resets

**Build:** the authoritative main tick ordering, persistent traces, the running reward baseline, and continuously preserved activity, adaptation, motor, and modulation states.

**Task:** keep the stationary clean associations. Introduce variable timing and gradually increase reward delay. Use one unresolved choice at a time.

**Compare:** episodic diagnostic, persistent activity with trace resets, and fully persistent activity plus traces. Name all three honestly.

**Exit:** above-chance acquisition remains measurable in the fully continuous condition, with no reset hooks firing except at birth. Trace magnitudes and bound occupancy remain interpretable.

**If it fails:** inspect cross-choice trace interference, feedback ordering, too-long traces, reward-baseline drift, and saturated motor activity. Do not paper over a failure by secretly restoring resets.

### M5 - Establish the adaptation/noise trade-off

**Build:** isolated reversals, mixed stable/volatile cues, and independently assigned feedback noise. Add acquisition, recovery, and stable-cue metrics.

**Run a fixed-rule sweep:** for example, `eta` in `{1e-4, 3e-4, 1e-3, 3e-3}` and `tau_e` in `{16, 32, 64, 128}`. These values are proposed development settings; adjust ranges only with a recorded reason.

**Look for:** a meaningful range where stronger updates recover faster after real changes but become more vulnerable to unreliable feedback or interference. It is also possible the task is so easy that one fixed setting works everywhere.

**Exit:** the always-on baseline has a characterized performance surface, the simple tabular comparator works, and hidden-state annotations remain analysis-only.

**Do not force the desired outcome:** if there is no trade-off under this environment, report that and revise the task transparently. Do not cherry-pick one weak fixed learning rate.

### M6 - Add gates before evolving them

**Build:** deterministic modulators, global and targeted projection heads, and event-aligned gate logging.

**Run controlled gate tests:** all gates zero, all gates one, all gates one half, one receiver enabled, and a manually supplied alternating gate sequence.

**Required checks:** zero gates prevent task-dependent plastic changes; constant gates rescale unclipped updates exactly; one targeted gate changes only incoming plastic edges of its receiver; changing modulator parameters cannot change actor behavior when plasticity is disabled.

The last check depends on the modulation-only architecture and paired actor noise. It is an especially useful regression test.

**Exit:** gate behavior matches arithmetic and architecture exactly. Only then enable evolutionary search.

### M7 - Evolve a small learning-control mechanism

**Build:** genome serialization, bounded decoding, population evaluation, elite selection, mutation, validation, and resumable search checkpoints.

**Start small:** 16 actor neurons, 2 modulators, a few cue types, and short training lifetimes. Search the gate projections first. Ensure the lifetime contains enough actual changes for modulation to affect fitness; inspect realized event counts.

**Search sanity test:** run the evolutionary engine on a trivial numerical objective with a known optimum before connecting it to the simulator. Also check that mutation changes decoded parameters and that all candidates use the same training batch.

**Exit:** multiple fresh outer seeds produce either reproducible improvements over a properly tuned fixed gate or a clearly documented null result. Evolutionary logs show no reuse of final-test seeds and no inherited lifetime offsets.

**If search stalls:** distinguish insufficient mutation, saturated gates, insufficient fitness signal, too-short lifetimes, and a genuinely strong fixed baseline. Do not immediately add neurons.

### M8 - Run the primary comparisons

**Build:** a suite runner for fixed, global, targeted, and nonplastic conditions; common seed manifests; an analysis pipeline; and outer-seed confidence intervals.

**Scale only after profiling:** move toward 60 actor neurons and 4 modulators where justified. A successful 16-neuron study is preferable to an unfinished 64-neuron search.

**Run:** fresh-seed generalization, the noise/hazard grid, isolated changes, and longer lifetimes. Keep genome selection tied to validation.

**Exit:** the report includes all outer seeds, failures, budgets, tuned baseline settings, and both observed reward and latent accuracy/regret.

**Claim boundary:** without the stronger optimized activity-only comparator, restrict conclusions to the tested locally plastic actor family.

### M9 - Test the mechanism causally

**Build:** branch-from-checkpoint interventions, fixed and replayed gates, spatial permutations, update-magnitude controls, and weight/state perturbations.

**Run:** predeclared intervention suites on selected frozen genomes and fresh lifetimes. Include both successful and ordinary seeds rather than only a showcase individual.

**Exit:** the report distinguishes a performance advantage from a supported explanation. Document simpler explanations that survive the interventions.

### M10 - Package the research result

**Deliver internally:** configuration files, seed manifests, code revision, selected genomes, representative complete checkpoints, raw event logs, derived metrics, figures, and a written interpretation with explicit limitations.

**Only now choose the next project direction:** more biological credit assignment, spiking dynamics, bodily control, or richer environments. Change one major axis at a time.

---

## 17. Test inventory and numerical checks

### 17.1 Environment unit tests

- A correct action receives reward 1 with zero noise; an incorrect action receives 0.
- A forced noise flip reverses either reward.
- Hazard zero never changes a mapping.
- Hazard one flips a mapping at every repeat presentation, but not at the first presentation.
- A reward uses the mapping saved at commitment.
- Delay one delivers at the next tick's start, not at the commitment tick's end.
- Reward zero sets `outcome_present=1` and `outcome_value=0`.
- The previous-action latch becomes visible on the tick after commitment.
- Each commitment creates exactly one consumed feedback event.
- A completed lifetime has no unresolved commitment.
- Agent-dependent random draws do not alter exogenous environment schedules.
- Observations contain no hidden labels or evaluation-only fields.

### 17.2 Neural and plasticity unit tests

- A one-edge network sends activity from `i` to `j`, never the reverse.
- All neurons use old state during one transition.
- Missing and nonplastic edges never acquire `P` updates.
- The same postsynaptic `xi[j]` is used on every edge entering `j`.
- With zero presynaptic activity, that transition's edge score is zero.
- With `delta=0`, task-dependent updates are zero.
- With gate zero or `eta=0`, `P` does not change.
- Doubling an unsaturated gate doubles its unclipped update.
- Adaptation is inert when its strength is zero.
- No event changes `W0`.
- Clipping is applied once in the documented order.
- Gates are sampled before current feedback integration in the main mode.
- A second delivery of the same feedback event is rejected.
- Traces are not reset after feedback in the persistent mode.

### 17.3 Hand-calculated golden update

Use this direct arithmetic fixture; it is independent of the ordinary configured time constants:

```text
alpha_h              = 0.5
presynaptic activity = 0.2
postsynaptic xi      = 0.4
sigma                = 0.1
old eligibility      = 0.3
lambda_e             = 0.9

score = 0.5 * 0.2 * 0.4 / 0.1 = 0.4
new eligibility = 0.9 * 0.3 + 0.4 = 0.67
```

At a later feedback event using that eligibility:

```text
reward               = 1.0
old reward baseline  = 0.6
beta_R               = 0.1
eta                  = 0.01
gate                 = 0.25
old P                = 0.1
bounds               = wide enough not to clip

delta = 1.0 - 0.6 = 0.4
raw update = 0.01 * 0.4 * 0.25 * 0.67 = 0.00067
new P = 0.10067
new reward baseline = 0.6 + 0.1 * 0.4 = 0.64
```

Compare within a tight floating-point tolerance. Also assert that the baseline update did not occur before calculating `delta`.

### 17.4 Conditional log-probability derivative test

Choose a fixed old state, one weight, a fixed sampled `h_new`, and all remaining parameters. Recompute the conditional Gaussian log probability after perturbing only that weight by `+eps` and `-eps`:

```text
numerical_score = (logp(W + eps) - logp(W - eps)) / (2 * eps)
```

Compare to the score formula. Use several `eps` values around `1e-6` to check numerical sensitivity. Do not resample `h_new` when evaluating the two log probabilities. Holding the same sample fixed is essential.

This test requires no automatic differentiation and is not part of lifetime learning.

### 17.5 Closed-form stochastic learning-direction test

Use one transition:

```text
mu = alpha * weight * input
h = mu + sigma * xi
reward = 1 if h > 0 else 0
score = alpha * input * xi / sigma
```

With standard-normal density `phi`, the exact derivative is:

```text
d E[reward] / d weight = (alpha * input / sigma) * phi(mu / sigma)
```

Use `alpha=0.2`, `input=0.7`, `weight=0.3`, and `sigma=0.4`. Here `mu=0.042`; the derivative is approximately `0.138862`.

Estimate the mean of `(reward - 0.5) * score` using many independent noise draws. Compute the standard error from the sampled terms. Require agreement within a prespecified tolerance, such as five standard errors plus a small numerical tolerance.

A rare statistical test failure is possible even with correct code. Pin the diagnostic seed and inspect a failure instead of rerunning until it passes. Also test the opposite target action and confirm the derivative changes sign.

### 17.6 Finite-horizon recurrent check

Use a two-neuron network, a short horizon, fixed initial state, fixed weights during the rollout, no trace decay, and a terminal reward. Compute the Monte Carlo mean reward under small positive and negative perturbations to one inherited weight.

Compare the finite-difference estimate with the mean terminal reward times the summed score for that edge. Use multiple perturbation magnitudes and uncertainty estimates. Short noisy trajectories can require many samples; this is a numerical diagnostic, not a production learning method.

Do not expect exact agreement for the full online clipped/gated update rule. That rule intentionally changes the estimator.

### 17.7 Replay and parallelism tests

- Identical seed/configuration/code revision reproduces the same reference trajectory.
- Splitting a run at a checkpoint reproduces uninterrupted continuation.
- Parallel candidate evaluation gives the same candidate scores as serial evaluation under a fixed deterministic reduction order.
- Changing log verbosity does not change random draws or behavior.
- Turning modulation off with learning disabled does not change actor behavior.
- Dense and sparse kernels agree within a declared tolerance on the same graph and noise draws.
- Reordering worker completion does not alter selection tie breaks.

Bitwise identity across different CPU architectures or math-library implementations is not guaranteed merely by using the same seed. Record the reference platform and distinguish bitwise from tolerance-based reproducibility.

### 17.8 Statistical negative controls

Run shuffled or uninformative reward conditions. A learner should not reliably infer random hidden mappings from feedback independent of correctness.

Also compare a condition with the correct trace rule to one that permutes postsynaptic perturbations among neurons. The latter is a destructive diagnostic; do not assume a particular failure magnitude, but investigate if it performs identically on all tasks.

---

## 18. Repository and interface design

### 18.1 Suggested structure

```text
learning-when-to-learn/
  spec.md
  README.md
  Cargo.toml
  Cargo.lock
  configs/
    debug_stationary.toml
    continuous_stationary.toml
    mixed_fixed.toml
    mixed_global.toml
    mixed_targeted.toml
    isolated_reversal.toml
    interventions.toml
  src/
    main.rs
    config.rs
    rng.rs
    environment/
      mod.rs
      schedule.rs
      observation.rs
      hidden_state.rs
    agent/
      mod.rs
      actor.rs
      modulator.rs
      plasticity.rs
      motor.rs
      genome.rs
      checkpoint.rs
    evolution/
      mod.rs
      population.rs
      mutation.rs
      evaluation.rs
    experiments/
      baseline.rs
      suite.rs
      intervention.rs
    logging/
      events.rs
      traces.rs
      manifest.rs
  tests/
    environment_contract.rs
    golden_updates.rs
    score_derivatives.rs
    event_order.rs
    replay.rs
    leakage.rs
  analysis/
    validate_logs.py
    aggregate.py
    plot_learning.py
    plot_reversals.py
    plot_interventions.py
  manifests/
    development.json
    validation.json
    final_test.json
  runs/
    <run_id>/
```

This is a target repository layout. The files and commands described here are to be implemented; this specification does not include an existing executable project.

### 18.2 Keep four responsibilities separate

**Environment:** generates observations and consequences.

**Agent:** advances dynamics and learns from allowed observations.

**Search:** selects inherited parameters using complete-lifetime scores.

**Evaluator:** computes hidden-truth metrics and performs deliberate interventions.

Do not put the true preferred action into a shared struct that every component receives just because it is convenient.

### 18.3 Example Rust boundary types

The following is an interface sketch using only standard Rust types. It is not the full simulator:

```rust
#[derive(Clone, Copy, Debug)]
pub struct Feedback {
    pub event_id: u64,   // Infrastructure only; not a neural feature.
    pub reward: f64,     // Observed outcome, not latent correctness.
}

#[derive(Clone, Debug)]
pub struct Observation {
    pub features: Vec<f64>,
    pub feedback: Option<Feedback>,
}

#[derive(Clone, Copy, Debug)]
pub struct MotorOutput {
    pub action_0: f64,
    pub action_1: f64,
}

#[derive(Clone, Debug)]
pub enum SimError {
    InvalidConfiguration(String),
    DuplicateFeedback(u64),
    NonFiniteState { tick: u64, component: String },
    InconsistentCheckpoint(String),
}

pub trait Agent {
    // Called at most once per delivered feedback event, before advance().
    fn apply_feedback(&mut self, event: Feedback) -> Result<(), SimError>;

    // One neural transition; must not apply feedback a second time.
    fn advance(&mut self, features: &[f64]) -> Result<MotorOutput, SimError>;
}

#[derive(Clone, Debug)]
pub struct RecurrentEdge {
    pub source: usize,
    pub receiver: usize,
    pub inherited_weight: f64,
    pub plastic_offset: f64,
    pub eligibility: f64,
    pub is_plastic: bool,
}
```

Put evaluator annotations in another module/type. Do not add `correct_action` to `Feedback` or `Observation`.

### 18.4 Agent implementation notes

Preallocate working buffers for old/new activity, drives, perturbations, and gates. Avoid allocating in the inner tick loop.

Draw a perturbation for every actor neuron on every tick regardless of which gates are open. Otherwise changing a gate could shift the RNG stream and confound paired comparisons.

Store a stable edge ordering, preferably grouped by receiving neuron. Read old activities while accumulating input, then perform neuron transitions, then update traces from the saved old activities and the receiving-neuron perturbations.

Store `W0` and `P` separately even if an optimized kernel caches `W0+P`. Update that cache only in one clearly tested location.

### 18.5 Dense first, sparse second

For the smallest debug network, a dense reference implementation is acceptable and easy to inspect. A sparse edge-list kernel should come later, with parity tests against the dense implementation.

At approximately 60 actor neurons and low connection probability, a sparse representation can avoid work on missing edges. Whether it is actually faster depends on implementation and hardware; benchmark rather than assume.

### 18.6 Suggested command interface

Implement commands along these lines:

```bash
cargo test
cargo run --release -- validate-config configs/debug_stationary.toml
cargo run --release -- simulate --config configs/debug_stationary.toml --seed 1
cargo run --release -- benchmark --config configs/mixed_fixed.toml
cargo run --release -- evolve --config configs/mixed_global.toml --outer-seed 1
cargo run --release -- evaluate --genome runs/example/genome.json --suite manifests/final_test.json
cargo run --release -- intervene --checkpoint runs/example/state.bin --config configs/interventions.toml
python analysis/validate_logs.py runs/example
python analysis/aggregate.py runs/study
```

These are proposed CLI contracts, not commands that already exist. Paths such as `runs/example` must be replaced by actual run paths after implementation.

### 18.7 Dependency policy

Use ordinary libraries for configuration, serialization, random distributions, and plotting. Pin versions and keep lockfiles. Avoid bringing in a deep-learning framework unless it provides a specific needed capability.

No autograd engine is required for the specified lifetime learning rule or the proposed evolutionary search. Numerical derivative checks remain outside the agent.

---

## 19. Configuration profiles

### 19.1 Design principle

All meaningful numerical and algorithmic choices belong in validated configuration, including reset policies and feedback ordering. Do not bury a research condition in a source-code constant.

Save the fully resolved configuration with every run, including defaults. A small user-facing override file is not sufficient provenance if defaults later change.

### 19.2 Complete starting debug profile

The following TOML is a proposed schema for a continuous clean-task debug run. Its values are starting choices:

```toml
schema_version = 1
profile_name = "debug_stationary"

[simulation]
dt = 1.0
precision = "f64"
warmup_ticks = 32
outcomes_per_lifetime = 2000
reset_policy = "birth_only"
feedback_order = "before_neural_transition"

[environment]
kind = "stationary_clean"
cue_count = 2
cue_encoding = "one_hot"
stable_fraction = 1.0
feedback_noise_values = [0.0]
volatile_hazard_values = [0.0]
hazard_clock = "cue_exposure"
quiet_ticks = [4, 4]
cue_ticks = 8
memory_gap_ticks = [0, 0]
response_ticks = 4
reward_delay_ticks = [1, 1]
feedback_ticks = 1
max_pending_choices = 1

[actor]
neuron_count = 16
motor_neurons_per_action = 2
edge_probability = 0.25
self_edges = false
recurrent_gain = 0.8
input_scale = 0.3
tau_h = 5.0
tau_a = 100.0
adaptation_strength = 0.0
noise_sigma = 0.05
motor_filter_tau = 3.0

[learning]
enabled = true
rule = "gaussian_transition_score"
plastic_mask = "all_recurrent_edges"
trace_policy = "persistent"
tau_e = 32.0
eta = 0.001
max_update = 0.01
plastic_bound = 0.5
plastic_decay = 0.0
reward_baseline_initial = 0.5
reward_baseline_beta = 0.02

[modulator]
mode = "fixed"
neuron_count = 2
tau_m = 20.0
ordinary_feedback_to_actor = false
gate_timing = "pre_outcome"
gate_bias_initial = 0.0
projection_init_std = 0.01

[evolution]
enabled = false
population_size = 32
elite_count = 8
mutation_std = 0.1
search_space = "gate_projection_only"

[logging]
event_log = true
trace_every_ticks = 10
full_trace_lifetimes = 1
record_raw_and_applied_updates = true
```

The exact finite-rollout score diagnostic needs a separate mode: fixed baseline during the rollout, no trace decay, fixed weights until the terminal event, and reset between independent diagnostic rollouts. Do not overload `birth_only` to secretly mean trial resets.

### 19.3 Main targeted-search profile

This is a larger starting profile, not an instruction to run it before the smaller milestones pass:

```toml
schema_version = 1
profile_name = "mixed_targeted"

[simulation]
dt = 1.0
precision = "f64"
warmup_ticks = 32
outcomes_per_lifetime = 2048
reset_policy = "birth_only"
feedback_order = "before_neural_transition"

[environment]
kind = "mixed_continual"
cue_count = 8
cue_encoding = "one_hot"
stable_fraction = 0.5
feedback_noise_values = [0.0, 0.1, 0.2]
volatile_hazard_values = [0.005, 0.02]
hazard_clock = "cue_exposure"
quiet_ticks = [8, 16]
cue_ticks = 16
memory_gap_ticks = [0, 8]
response_ticks = 8
reward_delay_ticks = [8, 24]
feedback_ticks = 1
max_pending_choices = 1

[actor]
neuron_count = 60
motor_neurons_per_action = 4
edge_probability = 0.15
self_edges = false
recurrent_gain = 0.8
input_scale = 0.3
tau_h = 5.0
tau_a = 100.0
adaptation_strength = 0.0
noise_sigma = 0.05
motor_filter_tau = 3.0

[learning]
enabled = true
rule = "gaussian_transition_score"
plastic_mask = "all_recurrent_edges"
trace_policy = "persistent"
tau_e = 64.0
eta = 0.001
max_update = 0.01
plastic_bound = 0.5
plastic_decay = 0.0
reward_baseline_initial = 0.5
reward_baseline_beta = 0.02

[modulator]
mode = "targeted"
neuron_count = 4
tau_m = 20.0
ordinary_feedback_to_actor = false
gate_timing = "pre_outcome"
gate_bias_initial = 0.0
projection_init_std = 0.01

[evolution]
enabled = true
population_size = 32
elite_count = 8
mutation_std = 0.1
search_space = "modulator_and_gate"
decoded_weight_limit = 2.0
lifetimes_per_candidate = 6
generations = 100
validation_every_generations = 10
validation_lifetimes = 24

[logging]
event_log = false
trace_every_ticks = 20
full_trace_lifetimes = 2
record_raw_and_applied_updates = true
```

The search profile disables full event streams for ordinary candidate evaluations. Enable event logging in selected validation/test replays; candidate-level scores and failure records must still be saved for every evaluation.

A six-lifetime training batch is only a starting compromise. Increase or stratify it if ranking noise is high. Do not launch the entire nominal search without a measured simulation-throughput estimate.

### 19.4 Configuration validation

Reject configurations with:

- nonpositive required time constants or `dt` other than 1 for this first implementation;
- zero or negative noise scale while the stochastic score is active;
- feedback noise outside `[0,0.5]` for this environment;
- invalid hazard probabilities;
- fewer actor neurons than required motor assignments;
- non-disjoint motor populations;
- reward delay below one tick;
- more than one pending choice in the first protocol;
- a gate mode inconsistent with its projection dimensions;
- trace resets enabled in a run labeled as persistent;
- both automatic weight decay and a claim of no other forgetting mechanism;
- elite count not strictly between zero and population size;
- missing seed namespace or unsupported schema version.

Also warn, rather than necessarily reject, when the chosen lifetime/hazard combination produces very few expected changes. Compute and log realized change counts for every lifetime.

### 19.5 Condition construction

Generate B4/B5/B6 configurations from one shared base with explicit overrides. Avoid maintaining three independently edited files that gradually diverge in unrelated parameters.

The fixed condition uses `mode="fixed"`; the global and targeted conditions use their respective gate heads. Their tuned learning rates may differ, but record the selection process and do not disguise it as identical hyperparameters.

---
## 20. Logging, replay, and checkpoints

### 20.1 Per-run manifest

Every run directory should contain:

```text
manifest.json              run identity, code revision, hardware, seed policy
resolved_config.toml       complete effective configuration
condition.json             model family and any interventions
search_summary.jsonl       generation-level scores and selection events
candidate_scores.jsonl     per-candidate/per-lifetime fitness and failures
genome.json                selected inherited parameters
metrics.json               evaluation summaries
```

Evaluation runs additionally contain event logs, selected neural traces, hidden-truth annotations, and complete checkpoints where needed.

Record the code revision and whether the working tree was modified. A revision hash alone is incomplete provenance when uncommitted changes affected the run.

### 20.2 Event-level record

For each evaluated outcome, store:

```text
run_id, condition_id, outer_seed, genome_id, lifetime_seed
choice_index, cue_observation_index, commit_tick, outcome_tick
action, reward, reward_baseline_before, delta
motor_margin_at_commit
gate_mean, gate_std, gate_min, gate_max
eligibility_L1, eligibility_L2
raw_update_L1, actual_update_L1, actual_update_L2
fraction_updates_clipped, fraction_offsets_at_bound
numerical_health_flags
```

Keep hidden annotations in an evaluator-only stream keyed by choice index:

```text
cue_id, target_at_commit, latent_correctness
noise_bit, epsilon, cue_hazard, cue_exposure_index
hidden_change_before_presentation, stable_or_volatile
```

Separate storage helps reinforce the agent/evaluator boundary. Analysis can join these streams; an agent must not read them.

### 20.3 Trace logging

For selected lifetimes, record actor activities, adaptation states, modulator states, gate vectors, motor outputs, and a stable sampled subset of edges' eligibility and plastic offsets.

Full edge traces across every tick of every evolutionary evaluation are unnecessary and may dominate storage and execution. During search, favor fitness summaries and sampled diagnostics. Re-run selected genomes on specified validation lifetimes to obtain full traces.

Interpret `full_trace_lifetimes` as a cap on deliberately selected detailed evaluation/replay lifetimes, not a request to record that many full traces for every candidate. The `record_raw_and_applied_updates` flag requires both kinds of summary norms and sampled-edge values, not necessarily every edge at every event.

### 20.4 Checkpoint cadence

Save search state at a regular generation interval and whenever a new validation-selected genome is accepted. Save complete lifetime checkpoints at predetermined intervention points.

Write checkpoint files atomically: write a temporary file, flush it, then rename. Include a schema version, configuration hash, genome hash, and content checksum. Reject incompatible or partial checkpoints rather than silently continuing with default values.

### 20.5 Deterministic seed derivation

Derive streams from an explicit tuple such as:

```text
(root_seed, namespace, outer_seed, lifetime_index, stream_name)
```

Use a documented deterministic derivation algorithm. Avoid language runtime hash functions whose behavior or randomization is not part of your reproducibility contract.

Record the pseudorandom generator algorithm and library version. For counter-based schedules, record the counter interpretation. For stateful generators, checkpoint their full state.

### 20.6 Automated log audit

Before analysis, validate:

- increasing tick and event indices;
- one feedback per commitment;
- no unexplained state resets;
- finite numeric values and gates within bounds;
- no updates on nonplastic edges;
- counts of hidden changes and noisy outcomes consistent with the realized schedule;
- no overlap among development, training, validation, and test seeds;
- correct number of complete lifetimes per outer seed;
- explicit records for failed or interrupted evaluations.

Do not fill missing scores with zero unless zero is the predefined failure score. Missing data and poor performance are different facts.

### 20.7 Research notebook

Keep a short append-only experiment log with the question, code/configuration hashes, what changed, observed result, interpretation, and next decision. Record null results and implementation mistakes.

A useful entry is: "Changing feedback ordering caused a spurious improvement; the corrected order removed it." That is more valuable than a directory full of unnamed successful plots.

---

## 21. Diagnostics and troubleshooting

| Symptom | First suspects | Next controlled action |
|---|---|---|
| No learning even on one constant target | Update sign, receiving/source mix-up, wrong noise scale | Run the one-neuron analytical test and inspect one event by hand |
| Clean episodic task learns but continuous task fails | Cross-choice trace contamination or ordering | Compare persistent activity with and without diagnostic trace resets |
| Reward-delay increase destroys learning | Trace attenuation and post-action variance | Sweep delay and `tau_e` while matching update scales |
| All gates collapse near zero | Damaging base learner or saturated gate initialization | Verify always-on acquisition; initialize half-open gates |
| Targeted beats fixed but not global | Timing sufficient; targeting unnecessary | Report the narrower result and run spatial controls |
| Targeted advantage disappears under rate matching | Effective learning-rate explanation | Compare against tuned fixed/global rates and update norms |
| Plastic offsets sit at their bounds | Excessive learning rate or biased drift | Reduce `eta`; inspect raw versus applied updates |
| Motor output is almost always the same | Saturation, weak cue drive, bad topology | Inspect cue responses, margins, and cue-to-motor reachability |
| Perfect performance on noisy fresh tasks immediately at birth | Leakage or mappings not actually randomized | Audit observation schema, birth seeds, and oracle separation |
| Evolution succeeds only on training schedules | Overfitting or insufficient lifetime diversity | Expand fresh-birth training batches and inspect validation gap |
| Evolutionary ranks change wildly on reevaluation | Noisy fitness or rare useful events | Increase/stratify lifetimes and log realized reversals |
| Gate/state reset harms everything equally | Nonspecific dynamical disruption | Add settling, copy controls, and targeted pathway interventions |
| Weight freezing has no effect | Activity-only memory or unused plasticity | Inspect actual `P`; run weight-transfer and optimized nonplastic controls |
| Performance degrades in longer lifetimes | Drift, bound saturation, stale credit | Plot offset occupancy, eligibility norms, and subgroup performance over time |
| Dense and sparse implementations disagree | Update ordering or edge indexing | Feed identical saved noise and compare each intermediate array |
| Parallel and serial runs disagree | Shared RNG, reduction ordering, nondeterministic selection | Isolate streams and deterministic tie-breaking |

### 21.1 Debug in the smallest possible system

Reduce in this order:

```text
one stochastic neuron and one weight
    -> two motor populations with a fixed cue
    -> two cues without recurrence
    -> small recurrent actor
    -> delayed outcomes
    -> persistent traces and state
    -> reversals/noise
    -> modulation
    -> evolution
```

Do not debug five novel mechanisms at once.

### 21.2 Separate learning failure from representation failure

Before blaming plasticity, ask whether the network receives enough information and whether relevant inputs reach the action pathway. Probe cue separation and short-term cue retention without claiming those probes are the agent's output.

An offline linear probe may help diagnose representation. It must not be used as a trained task-solving readout in the primary experiment.

### 21.3 Separate search failure from model failure

A finite evolutionary search may fail to discover a good circuit even when one exists. Useful checks include a simpler search space, an engineered positive-control gate, multiple mutation scales, and several independent outer seeds.

An engineered positive-control gate may use privileged change labels solely to test whether useful gating is possible in the architecture. Label it as an oracle intervention; never compare it as though the agent discovered that mechanism.

### 21.4 Avoid adding stabilizers without accounting for them

Global weight normalization, automatic rate homeostasis, clipping of neural states, and weight decay can all change learning and memory. They may be useful, but each is an additional mechanism.

When adding one, give it a named configuration field, explain its information access, test it independently, and rerun appropriate baselines. Otherwise a result attributed to modulation may actually depend on an unreported stabilizer.

---

## 22. Compute planning

### 22.1 Count work before scaling

Let:

```text
P = population size
G = number of generations
L = training lifetimes per candidate
T = ticks per lifetime
E = number of actor recurrent edges
N = actor neurons
M = modulator neurons
D = input dimensions
```

Training lifetime evaluations are approximately `P * G * L`, plus validation, tuning, and final testing. Total simulated ticks are approximately `P * G * L * T`.

A sparse tick includes roughly two actor-edge traversals: one for recurrent current and one for eligibility, plus actor input projections, modulator computations, nonlinearities, RNG, and logging. A useful work estimate is proportional to:

```text
T * (E + N*D + M*N + M*M + N*M)
```

The hidden constant matters. Measure actual throughput rather than predicting performance from the expression alone.

### 22.2 Main-profile arithmetic

With the proposed main timing, a choice cycle averages approximately:

```text
12 quiet + 16 cue + 4 memory gap + 8 response + 16 commitment-to-feedback
= 56 ticks
```

The commitment-to-feedback interval includes the feedback tick at its endpoint; do not add a second feedback tick. A delay of one means feedback at the next tick's start.

A 2,048-outcome lifetime is therefore roughly 114,720 ticks including a 32-tick warmup. The exact count varies with sampled durations.

For 60 actor neurons, no self-edges, and edge probability 0.15, expected recurrent edges are:

```text
60 * 59 * 0.15 = 531
```

The nominal search of 32 candidates, 100 generations, and 6 lifetimes each contains 19,200 lifetime evaluations, or roughly 2.2 billion ticks per outer seed before validation. Two actor-edge passes alone are then on the order of 2.3 trillion edge visits.

These are arithmetic estimates, not measured runtimes. They are a warning to benchmark and begin with the smaller profile, not a claim that the nominal search is inexpensive.

### 22.3 Scaling order

Reduce or increase work in a controlled order:

1. Verify correctness on the small actor and clean task.
2. Benchmark a complete representative lifetime with logging off and on.
3. Start gate-projection-only search.
4. Increase training diversity and independent seeds before making the actor much larger.
5. Increase lifetime length enough to contain meaningful adaptation events.
6. Open more inherited parameters only when the simpler search is characterized.

Shortening a lifetime can remove nearly all rare changes. Reducing the number of neurons is often a cleaner initial cost reduction than accidentally removing the phenomenon being studied.

### 22.4 Parallelism

Parallelize independent candidate/lifetime evaluations first. Keep each small simulation single-threaded initially. This avoids unnecessary synchronization inside a tiny recurrent network.

Use a bounded worker pool. Avoid nested parallelism that creates more active threads than intended. Aggregate results in a deterministic order before selection.

### 22.5 Logging budget

Store compact candidate scores for all search evaluations. Collect detailed event and neural traces for selected validation replays. This preserves auditability without writing every synaptic state for billions of ticks.

Never change a run's behavior depending on whether its log is enabled. Logging must be observational.

### 22.6 Budget fairness

Include development sweeps, baseline tuning, failed candidates, and validation in the reported budget. Equal generations with different population sizes or lifetime counts are not equal optimization budgets.

If a method gets a staged low-cost screening procedure, document it and consider applying an analogous procedure to the other methods. Do not make the favored model the only one that gets extensive tuning.

---

## 23. Extensions after the core experiment

### 23.1 Remove explicit perturbation access

Once the Gaussian-score version is understood, implement a separately sourced local rule that estimates useful fluctuations from neural state. Use a faithful reproduction harness for that rule before transferring it to the continuous environment.

Keep the task, gate architecture, and evaluation protocol fixed during this replacement. The question is whether the gating result survives a different credit-assignment mechanism, not whether an entirely redesigned system can work.

Miconi's paper and source repository are relevant starting points, but its published method and the score rule in this document are not interchangeable implementations. [R2], [R7]

### 23.2 Outcome-responsive gates

Use the stored-credit protocol in Section 8.4. Compare it directly with pre-outcome gates and with a simple reward-dependent scalar gate.

A performance improvement may reflect the extra information available at update time. Do not attribute all of it to richer recurrent computation.

### 23.3 Learned internal teaching signals

Replace the fixed running reward baseline with a small internal outcome predictor or let a circuit transform observable outcomes into a bounded teaching signal.

First identify the predictor's own learning rule and information boundary. Otherwise the system merely moves the attribution problem into an unspecified helper.

Continue computing evolutionary fitness from actual environmental rewards. A generated internal signal must not be able to define its own success.

### 23.4 Multiple unresolved choices

Permit actions whose rewards arrive after later decisions. This is a stronger temporal-credit problem.

Do not solve it by silently supplying a reward-to-cue pointer to the agent. The environment may use internal event IDs for bookkeeping, but any task-identifying feedback input must be explicitly part of the sensory interface.

Expect the simple persistent eligibility rule to encounter more interference. Compare any credit-buffer or compartment mechanism as an architectural addition.

### 23.5 Overlapping cue representations

Replace one-hot cues with fixed-dimensional dense vectors. Keep norms and pairwise similarities controlled. Randomly assign mappings independently of vector identity.

Test whether targeted plasticity still helps when cues share pathways. This is a useful next experiment if the simple task is solved by motor-afferent plasticity alone.

Do not infer perceptual abstraction from random-vector memorization. Use compositional or structured generalization tasks only when that becomes the explicit question.

### 23.6 Spiking neurons

A spiking version needs a new dynamics and learning specification. Thresholding and reset behavior change the stochastic model, so the Gaussian membrane score must not be copied unchanged into a different transition distribution.

Start with a published or independently derived rule for the chosen spiking model. Eligibility-propagation work is a relevant reference, but its learning signals and derivation differ from the simple score rule here. [R3]

Keep output as filtered activity from fixed motor populations. Compare matched tasks and resource budgets. The purpose is to test robustness to a different neuron abstraction, not to assume that spikes automatically improve learning.

### 23.7 Embodied demonstration

A manageable body extension is a one-dimensional or small two-dimensional agent with two continuous motor channels. Sensory inputs report local cues; actions change position; delayed environmental outcomes depend on reaching a goal or interacting with an object.

Use continuously filtered motor-population differences as force or turning commands. Do not discretize the recurrent network into separate inference calls for each movement.

Begin with fixed motor semantics, then test unannounced changes in sensorimotor mapping. This creates a meaningful adaptation problem without requiring a visually elaborate world.

The body introduces dynamics, exploration, collisions, and reward-design choices. Re-establish simple baselines before attributing success or failure to modulation.

### 23.8 Structural growth and evolution

Only after a fixed-topology result is interpretable should the genome mutate topology, learning-rule forms, or neuron counts. These changes enlarge the search and complicate matched controls.

Keep separate records of inherited topology and any within-lifetime structural changes. Do not describe topology search as a required ingredient of the first experiment.

---

## 24. Interpreting results and bounding claims

### 24.1 Result-to-claim map

| Observation | Supported interpretation | Not established |
|---|---|---|
| Reward improves within a lifetime | Experience-dependent adaptation | Synaptic learning is responsible |
| Freezing future updates harms reversal recovery | Continued plasticity is used by that trained individual | No nonplastic architecture could solve the task |
| Learned offsets transfer useful behavior to a fresh state | `P` contains behaviorally useful information | All memory resides in weights |
| Global gates beat tuned fixed plasticity | Internal rate regulation helps under tested conditions | Targeted modulation is necessary |
| Targeted gates beat global gates | Targeting helps in the tested models/budgets | A universal advantage of locality |
| Spatial/timing interventions remove an advantage | Learned assignment/timing matters in the trained circuit | Biological equivalence |
| Advantage disappears after update-magnitude matching | A scale explanation remains plausible | The project failed or learned nothing |
| Optimized activity-only agent matches performance | Synaptic updates are unnecessary for this task at this scale | Plasticity is generally useless |
| Gates correlate with hidden change points | Descriptive association | Causal change detection |
| Search finds nothing robust | No demonstrated advantage under the tested search | The hypothesis is impossible |

### 24.2 Wording for a positive result

A defensible result might be:

> Under the specified noisy reversal task, internally generated receiving-neuron gates reduced expected regret relative to tuned fixed-rate and independently evolved global-gate controls. Branch interventions indicate that correctly assigned modulation contributed beyond average update magnitude. The effect generalized to fresh lifetimes and the declared held-out condition.

This is an example of a future supported claim, not a report of results already obtained.

### 24.3 Wording for a null or simpler result

Equally useful outcomes include:

> A learned global gate accounted for the benefit; receiver-specific targeting did not improve performance at the tested scale.

or:

> Apparent learning was primarily maintained by recurrent activity, and synaptic plasticity was not needed for this task family.

or:

> The always-on local rule failed in the continuous condition despite passing finite-rollout score tests, identifying persistent-credit interference as the next problem to solve.

These statements identify what was tested and what was learned. They avoid turning an optimization or implementation limit into an unsupported universal conclusion.

### 24.4 Claims to avoid

Do not call the result a general intelligence, a realistic brain, a solution to biological credit assignment, the first self-modifying network, or proof that one learning paradigm replaces LLMs.

Do not claim biological plausibility solely because signals are named neurons, dopamine, or synapses. State exactly which information each update uses and which mechanisms were engineered.

### 24.5 What would make the project distinctive

The most credible contribution is a combination of a reproducible continuous task, a transparent local learning mechanism, strong simpler controls, and causal evidence about a specific learning-regulation trade-off.

The architectural ingredients alone are insufficient for a novelty claim. Before writing a paper, expand the literature review to targeted neuromodulation, continual adaptation under noisy feedback, meta-learned plasticity, and evolved learning rules. Record exact overlap and differences rather than relying on a memorable project name.

---

## 25. First implementation checklist

Use this as the immediate work queue. These are build tasks, not completed items.

- [ ] Create the repository and save this specification.
- [ ] Define validated configuration and deterministic RNG streams.
- [ ] Implement the phase schedule and one pending reward.
- [ ] Separate observable inputs from hidden evaluator annotations.
- [ ] Pass environment tests with random, constant-action, and oracle agents.
- [ ] Implement a double-buffered continuous actor and fixed motor pools.
- [ ] Implement the Gaussian conditional score and its derivative tests.
- [ ] Pass the hand-calculated eligibility/update fixture.
- [ ] Demonstrate clean-task learning in an explicitly labeled finite-rollout diagnostic.
- [ ] Add persistent traces and remove all non-birth resets in the main mode.
- [ ] Demonstrate continuous clean-task acquisition before adding modulation.
- [ ] Add noisy feedback and isolated reversals; characterize tuned fixed plasticity.
- [ ] Implement global/targeted gates and verify their arithmetic without evolution.
- [ ] Implement and validate the evolutionary engine on a simple numerical objective.
- [ ] Run a small gate-only search with fresh births and disjoint seed namespaces.
- [ ] Compare fixed, global, and targeted plasticity across independent outer seeds.
- [ ] Add the stronger activity-only comparator before making a broad mechanism claim.
- [ ] Run branch interventions and update-magnitude controls.
- [ ] Freeze the primary analysis and evaluate held-out tasks.
- [ ] Archive code, genomes, configurations, seed manifests, failures, and results.

**First success to aim for:** a tiny recurrent agent that learns two unknown associations from delayed outcomes, then continues learning without resetting its neural state. Everything else should build on that demonstrated capability.

---

## 26. Related work and source notes

Sources were checked while preparing this specification. This is a targeted implementation bibliography, not an exhaustive or guaranteed-current novelty review. Equations labeled as this document's model, task constructions, numeric defaults, and experimental designs are proposals or direct derivations rather than reported findings from these papers.

### [R1] Statistical score-function reinforcement learning

Ronald J. Williams. **Simple statistical gradient-following algorithms for connectionist reinforcement learning.** *Machine Learning* 8, 229-256 (1992). DOI: `10.1007/BF00992696`.

Relevant foundation: stochastic local-unit updates driven by reinforcement. This document separately derives the conditional Gaussian score for its own specified neuron transition. The paper is not evidence that the clipped, gated, changing-weight continuous system here is unbiased or guaranteed to learn.

### [R2] Delayed-reward recurrent learning

Thomas Miconi. **Biologically plausible learning in recurrent neural networks reproduces neural dynamics observed during cognitive tasks.** *eLife* 6:e20899 (2017). DOI: `10.7554/eLife.20899`.

Relevant foundation: recurrent networks trained using delayed rewards and local eligibility-like mechanisms. The biological-fluctuation method is a later replication target, not the exact baseline implemented by this specification.

### [R3] Eligibility propagation in spiking recurrent networks

Guillaume Bellec and colleagues. **A solution to the learning dilemma for recurrent networks of spiking neurons.** *Nature Communications* 11, 3625 (2020). DOI: `10.1038/s41467-020-17236-y`.

Relevant foundation: eligibility traces combined with learning signals in recurrent spiking networks. Do not conflate e-prop with the explicitly measured Gaussian perturbation score used here.

### [R4] Internally modulated plasticity

Thomas Miconi, Aditya Rawal, Jeff Clune, and Kenneth O. Stanley. **Backpropamine: training self-modifying neural networks with differentiable neuromodulated plasticity.** ICLR 2019; arXiv record `2002.10585` was posted in 2020.

Relevant prior art: networks whose generated signals regulate plasticity, optimized using gradient-based outer training. This is one reason internally controlled plasticity alone is not a novelty claim.

### [R5] Evolutionary discovery of recurrent spiking plasticity rules

Basile Confavreux, Everton J. Agnes, Friedemann Zenke, Henning Sprekeler, and Tim P. Vogels. **Balancing complexity, performance and plausibility to meta learn plasticity rules in recurrent spiking networks.** *PLOS Computational Biology* 21(4):e1012910 (April 24, 2025). DOI: `10.1371/journal.pcbi.1012910`.

Relevant prior art: evolutionary search over local plasticity rules and the difficulty introduced by richer search spaces and objectives. Its tasks and neuron models differ from the proposed noisy-reversal experiment.

### [R6] Adaptation in recurrent activity

Jane X. Wang and colleagues. **Prefrontal cortex as a meta-reinforcement learning system.** *Nature Neuroscience* 21, 860-868 (2018). DOI: `10.1038/s41593-018-0147-8`.

Relevant prior art: a recurrent system can implement learning-like adaptation through its activation dynamics. This motivates distinguishing activity memory from changing synapses and training a serious nonplastic comparator.

### [R7] Author implementation for Miconi's recurrent-learning work

**ThomasMiconi/BiologicallyPlausibleLearningRNN**, public source repository.

Useful implementation reference: the README distinguishes the paper's biologically motivated rule from an included node-perturbation variant. Pin a specific commit and inspect its task/reset conventions before using it as a reproduction target. Do not assume its implementation matches this specification's continuous protocol.

[R1]: https://link.springer.com/article/10.1007/BF00992696
[R2]: https://elifesciences.org/articles/20899
[R3]: https://www.nature.com/articles/s41467-020-17236-y
[R4]: https://arxiv.org/abs/2002.10585
[R5]: https://journals.plos.org/ploscompbiol/article?id=10.1371/journal.pcbi.1012910
[R6]: https://www.nature.com/articles/s41593-018-0147-8
[R7]: https://github.com/ThomasMiconi/BiologicallyPlausibleLearningRNN

---

## Closing principle

**Build the smallest system in which learning is real, the information boundary is clear, and the proposed mechanism can be experimentally removed. Then earn each additional claim with a controlled comparison.**
