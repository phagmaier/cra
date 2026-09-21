//! B0/B1/O1 harness checks (M0-10; spec 13.1/13.3).
//!
//! The oracle reaches latent accuracy exactly 1 (including under hazard-1
//! flips) with reward exactly `!noise_bit`; constant baselines commit only
//! their action; the random baseline conforms to the ordinary `Agent`
//! interface and reproduces its action sequence from seeds; and paired
//! lifetimes share cue/noise schedules while rewards follow each agent's
//! own action — never a shared forced reward.

#[allow(dead_code)]
#[path = "support/mod.rs"]
mod support;

use cra::experiments::baseline::{ConstantBaseline, RandomBaseline, run_oracle, run_ordinary};
use support::base_config;

fn noisy_config() -> cra::config::Config {
    let mut cfg = base_config();
    cfg.environment.kind = "stationary_noisy".to_owned();
    cfg.environment.feedback_noise_values = vec![0.1];
    cfg.simulation.outcomes_per_lifetime = 8;
    cfg
}

#[test]
fn oracle_latent_accuracy_is_exactly_one() {
    let cfg = noisy_config();
    cra::config::validate(&cfg).expect("valid");
    let summary = run_oracle(&cfg, 1, "development", 1, 0).expect("oracle run");
    assert_eq!(summary.choices.len(), 8);
    assert_eq!(summary.latent_accuracy(), 1.0);
    // Correct every time means the observed reward is exactly the
    // unflipped noise complement — no super-oracle scoring leak.
    for choice in &summary.choices {
        assert!(choice.correct);
        assert_eq!(choice.reward, f64::from(!choice.noise_bit));
    }
}

#[test]
fn oracle_tracks_hidden_flips() {
    // Hazard 1 on a single cue: the mapping flips before every repeat
    // presentation. The oracle must still be correct on every choice.
    let mut cfg = base_config();
    cfg.environment.cue_count = 1;
    cfg.environment.stable_fraction = 0.0;
    cfg.environment.volatile_hazard_values = vec![1.0];
    cfg.simulation.outcomes_per_lifetime = 6;
    cra::config::validate(&cfg).expect("valid");
    let summary = run_oracle(&cfg, 1, "development", 1, 0).expect("oracle run");
    assert_eq!(summary.latent_accuracy(), 1.0);
    // ... while the mapping genuinely moved underneath (not a static task).
    let actions: Vec<u8> = summary.choices.iter().map(|c| c.action).collect();
    assert!(
        actions.contains(&0) && actions.contains(&1),
        "oracle switched actions to follow flips: {actions:?}"
    );
}

#[test]
fn constant_baselines_commit_only_their_action() {
    let cfg = noisy_config();
    cra::config::validate(&cfg).expect("valid");
    for action in [0u8, 1u8] {
        let mut policy = ConstantBaseline::new(action).expect("valid constant");
        let summary = run_ordinary(&cfg, 1, "development", 1, 0, "constant", &mut policy)
            .expect("baseline run");
        assert_eq!(summary.choices.len(), 8);
        for choice in &summary.choices {
            assert_eq!(choice.action, action);
            // Reward accounting identity holds per choice.
            assert_eq!(choice.reward, f64::from(choice.correct ^ choice.noise_bit));
        }
    }
}

#[test]
fn random_baseline_reproduces_its_action_sequence_from_seeds() {
    let mut cfg = base_config();
    cfg.simulation.outcomes_per_lifetime = 64;
    cra::config::validate(&cfg).expect("valid");
    let run = || {
        let mut policy = RandomBaseline::new(1, "development", 1, 0).expect("random baseline");
        run_ordinary(&cfg, 1, "development", 1, 0, "random", &mut policy).expect("baseline run")
    };
    let first = run();
    let second = run();
    let actions: Vec<u8> = first.choices.iter().map(|c| c.action).collect();
    assert_eq!(
        actions,
        second.choices.iter().map(|c| c.action).collect::<Vec<_>>()
    );
    assert!(actions.iter().all(|&a| a <= 1));
    assert!(
        actions.contains(&0) && actions.contains(&1),
        "random baseline explores both actions over 64 choices"
    );
    // A different lifetime index draws a different action sequence.
    let mut other_policy = RandomBaseline::new(1, "development", 1, 3).expect("random baseline");
    let other = run_ordinary(&cfg, 1, "development", 1, 3, "random", &mut other_policy)
        .expect("baseline run");
    assert_ne!(
        actions,
        other.choices.iter().map(|c| c.action).collect::<Vec<_>>()
    );
}

#[test]
fn paired_lifetimes_share_noise_bits_but_not_rewards() {
    // Same seeds: identical exogenous schedule (cues and noise bits) for
    // the oracle and a constant baseline, while each reward follows its own
    // action through the production commit path — never a shared forced
    // reward.
    let cfg = noisy_config();
    cra::config::validate(&cfg).expect("valid");
    let oracle = run_oracle(&cfg, 1, "development", 1, 0).expect("oracle run");
    let mut constant = ConstantBaseline::new(0).expect("constant");
    let baseline = run_ordinary(&cfg, 1, "development", 1, 0, "constant", &mut constant)
        .expect("baseline run");

    let oracle_cues: Vec<usize> = oracle.choices.iter().map(|c| c.cue).collect();
    let baseline_cues: Vec<usize> = baseline.choices.iter().map(|c| c.cue).collect();
    assert_eq!(oracle_cues, baseline_cues, "same cue schedule");

    let oracle_noise: Vec<bool> = oracle.choices.iter().map(|c| c.noise_bit).collect();
    let baseline_noise: Vec<bool> = baseline.choices.iter().map(|c| c.noise_bit).collect();
    assert_eq!(oracle_noise, baseline_noise, "shared noise bits");

    let oracle_events: Vec<u64> = oracle.choices.iter().map(|c| c.event_id).collect();
    let baseline_events: Vec<u64> = baseline.choices.iter().map(|c| c.event_id).collect();
    assert_eq!(
        oracle_events, baseline_events,
        "same event identity sequence"
    );

    // Rewards differ exactly where actions differ, per the XOR accounting.
    let mut differed = 0;
    for (o, b) in oracle.choices.iter().zip(baseline.choices.iter()) {
        assert_eq!(o.reward, f64::from(o.correct ^ o.noise_bit));
        assert_eq!(b.reward, f64::from(b.correct ^ b.noise_bit));
        if o.action != b.action {
            differed += 1;
            assert_ne!(
                o.reward, b.reward,
                "same bit, different action -> different reward"
            );
        }
    }
    assert!(
        differed > 0,
        "constant-0 disagrees with the oracle somewhere"
    );
}

#[test]
fn ordinary_runner_delivers_feedback_before_the_transition() {
    use cra::environment::{Agent, Feedback, MotorOutput, SimError};
    use cra::experiments::baseline::OrdinaryPolicy;
    struct Recorder {
        received: u64,
        stepped: u64,
    }
    impl Agent for Recorder {
        fn apply_feedback(&mut self, event: Feedback) -> Result<(), SimError> {
            assert_eq!(event.event_id, self.received);
            self.received += 1;
            Ok(())
        }
        fn advance(&mut self, features: &[f64]) -> Result<MotorOutput, SimError> {
            if features[4] == 1.0 {
                self.stepped += 1;
                assert_eq!(
                    self.received, self.stepped,
                    "feedback must precede sensory transition"
                );
            }
            Ok(MotorOutput {
                action_0: 0.0,
                action_1: 0.0,
            })
        }
    }
    impl OrdinaryPolicy for Recorder {
        fn select_action(&mut self) -> u8 {
            0
        }
    }
    let cfg = base_config();
    let mut agent = Recorder {
        received: 0,
        stepped: 0,
    };
    run_ordinary(&cfg, 1, "development", 1, 0, "recorder", &mut agent).expect("run");
    assert_eq!(agent.received, cfg.simulation.outcomes_per_lifetime);
    assert_eq!(
        agent.stepped, agent.received,
        "includes final feedback transition"
    );
}

#[test]
fn baselines_reject_duplicate_and_invalid_feedback_without_consuming_it() {
    use cra::environment::{Agent, Feedback, SimError};
    let mut agents: Vec<Box<dyn Agent>> = vec![
        Box::new(ConstantBaseline::new(0).unwrap()),
        Box::new(RandomBaseline::new(1, "development", 1, 0).unwrap()),
    ];
    for agent in &mut agents {
        assert!(
            agent
                .apply_feedback(Feedback {
                    event_id: 0,
                    reward: f64::NAN
                })
                .is_err()
        );
        agent
            .apply_feedback(Feedback {
                event_id: 0,
                reward: 1.0,
            })
            .unwrap();
        assert!(matches!(
            agent.apply_feedback(Feedback {
                event_id: 0,
                reward: 0.0
            }),
            Err(SimError::DuplicateFeedback(0))
        ));
        agent
            .apply_feedback(Feedback {
                event_id: 1,
                reward: 0.0,
            })
            .unwrap();
        assert!(
            agent
                .apply_feedback(Feedback {
                    event_id: 0,
                    reward: 1.0
                })
                .is_err()
        );
    }
}
