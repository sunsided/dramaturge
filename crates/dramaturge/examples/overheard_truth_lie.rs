//! # The Broken Seal: A Dramaturge Dialogue Example
//!
//! This example demonstrates the complete feature set of the dramaturge dialogue system
//! through a playable scenario called "The Broken Seal".
//!
//! ## Scenario Overview
//!
//! In Greyford, a border town controlled by the Civic Wardens, a medicine shipment has
//! gone missing. The player talks to Mara Vey, a field medic. Unknown to the player,
//! Captain Orren Vale overhears the conversation from behind a privacy screen. Later,
//! when talking to Orren, the player can tell the truth, lie, or refuse to answer.
//! Orren already knows what Mara said.
//!
//! ## Features Demonstrated
//!
//! - **Typed Tags**: Custom enums implementing `DialogTag` (Moral, Skill, SpeechAct, Social)
//! - **Intentions**: Explicit and implicit dialogue intentions
//! - **Conditions**: Fact-based and relationship-based gating
//! - **Outcomes**: Fact setting, relationship adjustments
//! - **Hidden Facts**: NPC knowledge that persists without player awareness
//! - **TUI**: Interactive terminal interface with debug mode
//!
//! ## Run the Example
//!
//! ```bash
//! cargo run --example overheard_truth_lie
//! ```
//!
//! Press `d` during gameplay to toggle debug mode and see hidden facts.

use dramaturge::*;
use std::io::{self, Write};

// =============================================================================
// CHARACTER IDS
// =============================================================================

/// Entity IDs for characters in the scenario
mod entities {
    use dramaturge::EntityId;

    pub fn player() -> EntityId {
        EntityId::new(1)
    }
    pub fn mara() -> EntityId {
        EntityId::new(2)
    }
    pub fn orren() -> EntityId {
        EntityId::new(3)
    }
    #[allow(dead_code)]
    pub fn talen() -> EntityId {
        EntityId::new(4)
    } // Optional courier
}

// =============================================================================
// TYPED TAGS
// =============================================================================

/// Moral alignment tags for dialogue options.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[allow(dead_code)]
enum Moral {
    /// Acts with compassion and idealism
    Paragon,
    /// Acts with practicality and compromise
    Pragmatic,
    /// Acts with ruthless efficiency
    Renegade,
}

/// Skill-based tags indicating what expertise the option uses.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[allow(dead_code)]
enum Skill {
    /// Medical knowledge and empathy
    Medic,
    /// Investigation and evidence analysis
    Investigator,
    /// Coercion and threats
    Intimidation,
    /// Street knowledge and deception
    Streetwise,
}

/// Speech act tags describing the communicative purpose.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[allow(dead_code)]
enum SpeechAct {
    /// Comforting someone in distress
    Comfort,
    /// Making an accusation
    Accuse,
    /// Making a promise
    Promise,
    /// Speaking truthfully
    Truth,
    /// Deceiving
    Lie,
    /// Deflecting a question
    Deflect,
    /// Asking for information
    Ask,
}

/// Social context tags for dialogue options.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[allow(dead_code)]
enum Social {
    /// Shows loyalty to a faction
    FactionLoyalty,
    /// Shared in confidence
    Confidential,
    /// Publicly known information
    Public,
}

/// Unified tag enum wrapping all tag types.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Tag {
    Moral(Moral),
    Skill(Skill),
    SpeechAct(SpeechAct),
    Social(Social),
}

impl DialogTag for Tag {
    fn axis(&self) -> &str {
        match self {
            Tag::Moral(_) => "moral",
            Tag::Skill(_) => "skill",
            Tag::SpeechAct(_) => "speech",
            Tag::Social(_) => "social",
        }
    }

    fn key(&self) -> &str {
        match self {
            Tag::Moral(m) => match m {
                Moral::Paragon => "paragon",
                Moral::Pragmatic => "pragmatic",
                Moral::Renegade => "renegade",
            },
            Tag::Skill(s) => match s {
                Skill::Medic => "medic",
                Skill::Investigator => "investigator",
                Skill::Intimidation => "intimidation",
                Skill::Streetwise => "streetwise",
            },
            Tag::SpeechAct(s) => match s {
                SpeechAct::Comfort => "comfort",
                SpeechAct::Accuse => "accuse",
                SpeechAct::Promise => "promise",
                SpeechAct::Truth => "truth",
                SpeechAct::Lie => "lie",
                SpeechAct::Deflect => "deflect",
                SpeechAct::Ask => "ask",
            },
            Tag::Social(s) => match s {
                Social::FactionLoyalty => "faction_loyalty",
                Social::Confidential => "confidential",
                Social::Public => "public",
            },
        }
    }
}

impl From<Tag> for DialogTagKind {
    fn from(tag: Tag) -> Self {
        DialogTagKind::new(tag.axis(), tag.key())
    }
}

// =============================================================================
// FACT IDS
// =============================================================================

/// Fact IDs for visible player knowledge
mod facts {
    use dramaturge::FactId;

    /// Player has talked to Mara
    pub fn talked_to_mara() -> FactId {
        FactId::new("TalkedToMara")
    }

    /// Mara revealed the medicine was diverted
    pub fn mara_revealed_diversion() -> FactId {
        FactId::new("MaraRevealedDiversion")
    }

    /// Medicine was diverted to refugees
    pub fn medicine_diverted_to_refugees() -> FactId {
        FactId::new("MedicineDivertedToRefugees")
    }

    /// Player promised Mara discretion
    pub fn player_promised_mara_discretion() -> FactId {
        FactId::new("PlayerPromisedMaraDiscretion")
    }

    /// Player comforted Mara
    pub fn player_comforted_mara() -> FactId {
        FactId::new("PlayerComfortedMara")
    }

    /// Player threatened Mara
    pub fn player_threatened_mara() -> FactId {
        FactId::new("PlayerThreatenedMara")
    }

    /// Player refused discretion to Mara
    pub fn player_refused_discretion() -> FactId {
        FactId::new("PlayerRefusedDiscretion")
    }

    /// Player told Orren the truth
    pub fn player_told_orren_truth() -> FactId {
        FactId::new("PlayerToldOrrenTruth")
    }

    /// Player lied to Orren
    pub fn player_lied_to_orren() -> FactId {
        FactId::new("PlayerLiedToOrren")
    }

    /// Player refused to betray Mara to Orren
    pub fn player_refused_to_betray_mara() -> FactId {
        FactId::new("PlayerRefusedToBetrayMara")
    }

    /// Orren respects player's honesty
    pub fn orren_respects_player_honesty() -> FactId {
        FactId::new("OrrenRespectsPlayerHonesty")
    }

    /// Orren softened his report
    pub fn orren_softens_report() -> FactId {
        FactId::new("OrrenSoftensReport")
    }

    /// Orren views player as manipulative
    pub fn orren_views_player_as_manipulative() -> FactId {
        FactId::new("OrrenViewsPlayerAsManipulative")
    }

    /// Player gave half-truth to Orren (omission)
    pub fn player_gave_half_truth_to_orren() -> FactId {
        FactId::new("PlayerGaveHalfTruthToOrren")
    }

    /// Orren pretends to believe player (secretly knows better)
    pub fn orren_pretends_to_believe() -> FactId {
        FactId::new("OrrenPretendsToBelieve")
    }

    /// Orren is plotting to test player further
    pub fn orren_plotting_to_test_player() -> FactId {
        FactId::new("OrrenPlottingToTestPlayer")
    }

    /// Mara refused to give player any information
    pub fn mara_refused_to_talk() -> FactId {
        FactId::new("MaraRefusedToTalk")
    }

    /// Player learned nothing from Mara
    pub fn player_learned_nothing() -> FactId {
        FactId::new("PlayerLearnedNothing")
    }
}

/// Hidden fact IDs (NPC knowledge)
mod hidden_facts {
    use dramaturge::FactId;

    /// Orren overheard the Mara conversation
    pub fn orren_overheard_mara_conversation() -> FactId {
        FactId::new("OrrenOverheardMaraConversation")
    }

    /// Orren knows medicine was diverted
    pub fn orren_knows_medicine_was_diverted() -> FactId {
        FactId::new("OrrenKnowsMedicineWasDiverted")
    }

    /// Orren knows player promised discretion
    pub fn orren_knows_player_promised_discretion() -> FactId {
        FactId::new("OrrenKnowsPlayerPromisedDiscretion")
    }

    /// Orren knows player threatened Mara
    pub fn orren_knows_player_threatened_mara() -> FactId {
        FactId::new("OrrenKnowsPlayerThreatenedMara")
    }

    /// Orren caught player in a lie
    pub fn orren_caught_player_lie() -> FactId {
        FactId::new("OrrenCaughtPlayerLie")
    }
}

// =============================================================================
// RELATIONSHIP AXES
// =============================================================================

/// Relationship axis identifiers
mod relationships {
    use dramaturge::RelationshipAxisId;

    /// Trust axis for relationships
    pub fn trust() -> RelationshipAxisId {
        RelationshipAxisId::new("trust")
    }

    /// Standing axis for faction reputation
    pub fn standing() -> RelationshipAxisId {
        RelationshipAxisId::new("standing")
    }
}

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Creates a tag from a Tag enum
fn tag(t: Tag) -> DialogTagKind {
    t.into()
}

/// Helper to create a relationship outcome
fn add_relationship(source: EntityId, target: EntityId, delta: i32) -> Outcome {
    Outcome::AddRelationship {
        source,
        target,
        axis: relationships::trust(),
        delta,
    }
}

/// Helper to create a standing outcome (stored as relationship with faction entity)
fn add_standing(entity: EntityId, delta: i32) -> Outcome {
    // Standing is represented as a relationship to the entity itself
    Outcome::AddRelationship {
        source: entities::player(),
        target: entity,
        axis: relationships::standing(),
        delta,
    }
}

/// Helper to create a set fact outcome
fn set_fact_world(fact_id: FactId) -> Outcome {
    Outcome::SetFact {
        scope: FactScope::World,
        fact: Fact::bool(fact_id.0.clone(), true),
    }
}

/// Helper to create a hidden fact outcome (stored as character-scoped fact for Orren)
fn set_hidden_fact(fact_id: FactId) -> Outcome {
    Outcome::SetFact {
        scope: FactScope::Character(CharacterId::new(entities::orren())),
        fact: Fact::bool(fact_id.0.clone(), true),
    }
}

/// Helper to check if a fact exists condition
fn has_fact(fact_id: FactId) -> Condition {
    Condition::FactExists {
        scope: FactScope::World,
        fact_id,
    }
}

/// Helper to check relationship condition
#[allow(dead_code)]
fn trust_at_least(target: EntityId, value: i32) -> Condition {
    Condition::RelationshipAtLeast {
        source: entities::player(),
        target,
        axis: relationships::trust(),
        value,
    }
}

// =============================================================================
// DIALOGUE NODE IDS
// =============================================================================

fn node_mara_intro() -> DialogueNodeId {
    DialogueNodeId::new(1)
}
fn node_mara_confession_soft() -> DialogueNodeId {
    DialogueNodeId::new(2)
}
fn node_mara_confession_guarded() -> DialogueNodeId {
    DialogueNodeId::new(3)
}
fn node_mara_confession_forced() -> DialogueNodeId {
    DialogueNodeId::new(4)
}
fn node_mara_promise_choice() -> DialogueNodeId {
    DialogueNodeId::new(5)
}
fn node_mara_refused() -> DialogueNodeId {
    DialogueNodeId::new(6)
}
fn node_orren_intro() -> DialogueNodeId {
    DialogueNodeId::new(10)
}
fn node_orren_truth_response() -> DialogueNodeId {
    DialogueNodeId::new(11)
}
fn node_orren_lie_response() -> DialogueNodeId {
    DialogueNodeId::new(12)
}
fn node_orren_refuse_response() -> DialogueNodeId {
    DialogueNodeId::new(13)
}
fn node_orren_pretends_belief() -> DialogueNodeId {
    DialogueNodeId::new(14)
}
#[allow(dead_code)]
fn node_end() -> DialogueNodeId {
    DialogueNodeId::new(100)
}

// =============================================================================
// GAME STATE
// =============================================================================

/// Tracks the current game state
struct GameState {
    /// The world state containing facts, relationships, and events
    world: DialogWorldState,
    /// Current node ID in the dialogue graph
    current_node: DialogueNodeId,
    /// Whether debug mode is enabled (shows hidden facts)
    debug_mode: bool,
    /// The dialogue graph containing all nodes and options
    graph: DialogueGraph,
    /// Current speaker name for display
    current_speaker: &'static str,
}

impl GameState {
    fn new() -> Self {
        let mut world = DialogWorldState::new();

        // Initialize relationships
        world.add_relationship(
            entities::player(),
            entities::mara(),
            relationships::trust(),
            0,
        );
        world.add_relationship(
            entities::player(),
            entities::orren(),
            relationships::trust(),
            0,
        );

        // Initialize faction standings
        world.add_relationship(
            entities::player(),
            entities::orren(),
            relationships::standing(),
            0,
        );
        world.add_relationship(
            entities::player(),
            entities::mara(),
            relationships::standing(),
            0,
        );

        let graph = build_dialogue_graph();

        Self {
            world,
            current_node: node_mara_intro(),
            debug_mode: false,
            graph,
            current_speaker: "Mara Vey",
        }
    }

    /// Get the current node from the graph
    fn current_node(&self) -> Option<&DialogueNode> {
        self.graph.nodes.iter().find(|n| n.id == self.current_node)
    }

    /// Get available options for the current node (filtered by conditions)
    fn available_options(&self) -> Vec<&DialogueOption> {
        let evaluator = DefaultConditionEvaluator;
        let node = match self.current_node() {
            Some(n) => n,
            None => return vec![],
        };

        node.options
            .iter()
            .filter(|opt| {
                opt.conditions
                    .iter()
                    .all(|c| evaluator.evaluate(c, &self.world))
            })
            .collect()
    }

    /// Apply outcomes from choosing an option
    fn apply_option(&mut self, option_index: usize) {
        // First, collect the data we need from the option
        let option_data = {
            let options = self.available_options();
            if option_index >= options.len() {
                return;
            }
            let option = options[option_index];
            (
                option.id,
                option.tags.clone(),
                option.intentions.clone(),
                option.outcomes.clone(),
                option.target.clone(),
            )
        };

        let (option_id, tags, intentions, outcomes, target) = option_data;
        let is_lying = tags
            .iter()
            .any(|t| *t.axis == *"speech" && *t.key == *"lie");
        let mut applier = DefaultOutcomeApplier;

        // Apply all outcomes
        for outcome in &outcomes {
            applier.apply(outcome, &mut self.world);
        }

        // Emit dialog option chosen event
        let event_kind = EventKind::DialogOptionChosen {
            node_id: self.current_node,
            option_id,
            tags,
            intentions,
        };
        self.world.emit_event(event_kind);

        // Handle special triggers for hidden observation
        self.handle_hidden_observation(&outcomes, is_lying);

        // Handle conditional outcomes for truth-telling (Option F)
        self.handle_truth_consequences(&target);

        // Transition to target node
        match &target {
            DialogTarget::Node(node_id) => {
                self.current_node = *node_id;
                self.update_speaker();

                // Apply outcomes_on_enter for the new node
                self.apply_node_outcomes(*node_id);
            }
            DialogTarget::End => {
                self.show_ending();
                std::process::exit(0);
            }
            _ => {}
        }
    }

    /// Apply outcomes_on_enter for a given node
    fn apply_node_outcomes(&mut self, node_id: DialogueNodeId) {
        if let Some(node) = self.graph.nodes.iter().find(|n| n.id == node_id) {
            let mut applier = DefaultOutcomeApplier;
            for outcome in &node.outcomes_on_enter {
                applier.apply(outcome, &mut self.world);
            }
        }
    }

    /// Handle hidden observation mechanics (Orren overhearing)
    fn handle_hidden_observation(&mut self, outcomes: &[Outcome], is_lying: bool) {
        // Orren overhears whenever player is talking to Mara
        if self.current_node.0 >= 1 && self.current_node.0 <= 9 {
            // This is a Mara node - Orren might overhear
            let mut applier = DefaultOutcomeApplier;

            // Set hidden fact that Orren overheard
            applier.apply(
                &set_hidden_fact(hidden_facts::orren_overheard_mara_conversation()),
                &mut self.world,
            );

            // Orren learns specific facts based on the player's choices
            if outcomes.iter().any(|o| matches!(o, Outcome::SetFact { fact, .. } if fact.id.0.as_ref() == "MedicineDivertedToRefugees")) {
                applier.apply(&set_hidden_fact(hidden_facts::orren_knows_medicine_was_diverted()), &mut self.world);
            }

            if outcomes.iter().any(|o| matches!(o, Outcome::SetFact { fact, .. } if fact.id.0.as_ref() == "PlayerPromisedMaraDiscretion")) {
                applier.apply(&set_hidden_fact(hidden_facts::orren_knows_player_promised_discretion()), &mut self.world);
            }

            if outcomes.iter().any(|o| matches!(o, Outcome::SetFact { fact, .. } if fact.id.0.as_ref() == "PlayerThreatenedMara")) {
                applier.apply(&set_hidden_fact(hidden_facts::orren_knows_player_threatened_mara()), &mut self.world);
            }
        }

        // Check if player is lying to Orren when he overheard the conversation
        // The blatant lie "Mara found crates missing" is caught if Orren overheard ANYTHING
        // because he knows Mara confessed to diverting them
        if self.current_node == node_orren_intro() {
            let orren_overheard = self.world.fact_exists(
                &FactScope::Character(CharacterId::new(entities::orren())),
                &hidden_facts::orren_overheard_mara_conversation(),
            );

            if is_lying && orren_overheard {
                let mut applier = DefaultOutcomeApplier;
                applier.apply(
                    &set_hidden_fact(hidden_facts::orren_caught_player_lie()),
                    &mut self.world,
                );
                applier.apply(
                    &set_fact_world(facts::orren_views_player_as_manipulative()),
                    &mut self.world,
                );
            }
        }
    }

    /// Handle conditional consequences for telling the truth to Orren
    /// - If player promised Mara discretion: decrease Mara standing (broken promise)
    /// - If player intimidated Mara: no additional decrease (already damaged)
    fn handle_truth_consequences(&mut self, target: &DialogTarget) {
        // Only apply when transitioning to truth response node
        match target {
            DialogTarget::Node(node_id) if *node_id == node_orren_truth_response() => {
                let mut applier = DefaultOutcomeApplier;

                // Check if player promised discretion
                let promised_discretion = self
                    .world
                    .fact_exists(&FactScope::World, &facts::player_promised_mara_discretion());

                // Check if player intimidated Mara
                let intimidated_mara = self
                    .world
                    .fact_exists(&FactScope::World, &facts::player_threatened_mara());

                if promised_discretion && !intimidated_mara {
                    // Broke promise - Mara feels betrayed
                    applier.apply(
                        &add_relationship(entities::player(), entities::mara(), -15),
                        &mut self.world,
                    );
                    applier.apply(&add_standing(entities::mara(), -10), &mut self.world);
                    // Emit a custom event to track this
                    self.world.emit_event(EventKind::Custom {
                        kind: "BrokePromiseToMara".into(),
                    });
                } else if promised_discretion && intimidated_mara {
                    // Broke promise but already intimidated - smaller additional hit
                    applier.apply(
                        &add_relationship(entities::player(), entities::mara(), -5),
                        &mut self.world,
                    );
                    self.world.emit_event(EventKind::Custom {
                        kind: "BrokePromiseButAlreadyHostile".into(),
                    });
                } else if intimidated_mara {
                    // No promise made, but already intimidated - no change
                    self.world.emit_event(EventKind::Custom {
                        kind: "TruthAfterIntimidation".into(),
                    });
                } else {
                    // No promise, no intimidation - neutral truth
                    // Small positive for honesty even if it hurts Mara
                    applier.apply(
                        &add_relationship(entities::player(), entities::mara(), 5),
                        &mut self.world,
                    );
                }
            }
            _ => {}
        }
    }

    /// Update the current speaker based on node
    fn update_speaker(&mut self) {
        self.current_speaker = match self.current_node.0 {
            1..=9 => "Mara Vey",
            10..=20 => "Captain Orren Vale",
            _ => "Unknown",
        };
    }

    /// Display the ending summary
    fn show_ending(&self) {
        clear_screen();
        println!("╔══════════════════════════════════════════════════════════════════╗");
        println!("║                      THE BROKEN SEAL - END                       ║");
        println!("╚══════════════════════════════════════════════════════════════════╝");
        println!();
        println!("Your choices have shaped the story:");
        println!();

        // Show final state
        let mara_trust = self
            .world
            .get_relationship(
                entities::player(),
                entities::mara(),
                &relationships::trust(),
            )
            .unwrap_or(0);
        let orren_trust = self
            .world
            .get_relationship(
                entities::player(),
                entities::orren(),
                &relationships::trust(),
            )
            .unwrap_or(0);

        println!(
            "  Final Trust with Mara Vey:     {}",
            format_trust(mara_trust)
        );
        println!(
            "  Final Trust with Orren Vale:   {}",
            format_trust(orren_trust)
        );
        println!();

        // Show key facts
        println!("Key Events:");
        if self
            .world
            .fact_exists(&FactScope::World, &facts::player_told_orren_truth())
        {
            let promised = self
                .world
                .fact_exists(&FactScope::World, &facts::player_promised_mara_discretion());
            let intimidated = self
                .world
                .fact_exists(&FactScope::World, &facts::player_threatened_mara());

            if promised && !intimidated {
                println!("  ✓ You told Orren the truth - but you broke your promise to Mara!");
            } else if promised && intimidated {
                println!(
                    "  ✓ You told Orren the truth - Mara expected nothing better after your threats"
                );
            } else if intimidated {
                println!("  ✓ You told Orren the truth - Mara already feared you");
            } else {
                println!("  ✓ You told Orren the truth about Mara");
            }
        }
        if self
            .world
            .fact_exists(&FactScope::World, &facts::player_lied_to_orren())
        {
            // Check if Orren overheard the conversation - if he did, he knows you're lying
            let orren_overheard = self.world.fact_exists(
                &FactScope::Character(CharacterId::new(entities::orren())),
                &hidden_facts::orren_overheard_mara_conversation(),
            );
            if orren_overheard {
                println!(
                    "  ✗ You lied to Orren - and he caught you! (he overheard the whole conversation)"
                );
            } else {
                println!("  ✗ You lied to Orren - but he didn't know");
            }
        }
        if self
            .world
            .fact_exists(&FactScope::World, &facts::player_gave_half_truth_to_orren())
        {
            if self
                .world
                .fact_exists(&FactScope::World, &facts::orren_pretends_to_believe())
            {
                println!("  ⚠ You gave Orren a half-truth - he PRETENDS to believe you");
                if self
                    .world
                    .fact_exists(&FactScope::World, &facts::orren_plotting_to_test_player())
                {
                    println!("    (He's now plotting to test your loyalty...)");
                }
            } else {
                println!("  ⚠ You gave Orren a half-truth");
            }
        }
        if self
            .world
            .fact_exists(&FactScope::World, &facts::player_refused_to_betray_mara())
        {
            println!("  ✓ You kept Mara's secret from Orren");
        }
        if self
            .world
            .fact_exists(&FactScope::World, &facts::mara_refused_to_talk())
        {
            println!("  ✗ Mara refused to talk to you - you learned nothing");
            if self
                .world
                .fact_exists(&FactScope::World, &facts::player_lied_to_orren())
                || self
                    .world
                    .fact_exists(&FactScope::World, &facts::player_gave_half_truth_to_orren())
            {
                println!("    (You bluffed to Orren with no real information)");
            }
        }

        println!();
        println!("Press Enter to exit...");
        let _ = read_line();
    }
}

fn format_trust(value: i32) -> String {
    match value {
        i32::MIN..=-20 => format!("{} (Hostile)", value),
        -19..=-5 => format!("{} (Wary)", value),
        -4..=4 => format!("{} (Neutral)", value),
        5..=19 => format!("{} (Cordial)", value),
        20.. => format!("{} (Trusted)", value),
    }
}

// =============================================================================
// DIALOGUE GRAPH CONSTRUCTION
// =============================================================================

fn build_dialogue_graph() -> DialogueGraph {
    let mut graph = DialogueGraph::new();

    // =========================================================================
    // SCENE 1: MARA INTRO
    // =========================================================================
    let mut mara_intro = DialogueNode::new(
        1,
        CharacterId::new(entities::mara()),
        DialogText::inline(
            "If you came about the missing shipment, I already told the Wardens everything I can.",
        ),
    );

    // Option A: Medic Comfort Path
    let mut opt_a = DialogueOption::new(
        1,
        DialogText::inline(
            "[Medic] You treated someone with that medicine, didn't you? I'm not here to punish that.",
        ),
        DialogTarget::Node(node_mara_confession_soft()),
        node_mara_intro(),
    );
    opt_a.tags = vec![
        tag(Tag::Moral(Moral::Paragon)),
        tag(Tag::Skill(Skill::Medic)),
        tag(Tag::SpeechAct(SpeechAct::Comfort)),
    ];
    opt_a.intentions = DialogIntentions::new()
        .with_explicit(Intention::Custom("GainTrust".into()))
        .with_implicit(Intention::Custom("ProtectVulnerable".into()))
        .with_implicit(Intention::Custom("ExtractConfessionSoftly".into()));
    opt_a.outcomes = vec![
        add_relationship(entities::player(), entities::mara(), 15),
        add_standing(entities::mara(), 5),
        set_fact_world(facts::player_comforted_mara()),
    ];

    // Option B: Investigator Path
    let mut opt_b = DialogueOption::new(
        2,
        DialogText::inline(
            "[Investigator] The ledger says three crates vanished after your signature.",
        ),
        DialogTarget::Node(node_mara_confession_guarded()),
        node_mara_intro(),
    );
    opt_b.tags = vec![
        tag(Tag::Moral(Moral::Pragmatic)),
        tag(Tag::Skill(Skill::Investigator)),
        tag(Tag::SpeechAct(SpeechAct::Ask)),
    ];
    opt_b.intentions = DialogIntentions::new()
        .with_explicit(Intention::Custom("ExtractInformation".into()))
        .with_implicit(Intention::Custom("PressureWitness".into()));

    // Option C: Renegade Pressure Path
    let mut opt_c = DialogueOption::new(
        3,
        DialogText::inline("[Intimidation] Don't waste my time. Tell me where the crates went."),
        DialogTarget::Node(node_mara_confession_forced()),
        node_mara_intro(),
    );
    opt_c.tags = vec![
        tag(Tag::Moral(Moral::Renegade)),
        tag(Tag::Skill(Skill::Intimidation)),
        tag(Tag::SpeechAct(SpeechAct::Accuse)),
    ];
    opt_c.intentions = DialogIntentions::new()
        .with_explicit(Intention::Intimidate)
        .with_implicit(Intention::Custom("CloseCaseQuickly".into()));
    opt_c.outcomes = vec![
        add_relationship(entities::player(), entities::mara(), -15),
        add_standing(entities::mara(), -5),
        set_fact_world(facts::player_threatened_mara()),
    ];

    // Option C2: Aggressive Accusation - causes Mara to shut down
    let mut opt_c2 = DialogueOption::new(
        20,
        DialogText::inline(
            "[Accuse] You're a thief and a liar. I should drag you to the stocks right now.",
        ),
        DialogTarget::Node(node_mara_refused()),
        node_mara_intro(),
    );
    opt_c2.tags = vec![
        tag(Tag::Moral(Moral::Renegade)),
        tag(Tag::SpeechAct(SpeechAct::Accuse)),
    ];
    opt_c2.intentions = DialogIntentions::new()
        .with_explicit(Intention::Intimidate)
        .with_implicit(Intention::Custom("ForceConfession".into()));
    opt_c2.outcomes = vec![
        add_relationship(entities::player(), entities::mara(), -25),
        set_fact_world(facts::player_threatened_mara()),
    ];

    mara_intro.options.push(opt_a);
    mara_intro.options.push(opt_b);
    mara_intro.options.push(opt_c);
    mara_intro.options.push(opt_c2);
    graph.nodes.push(mara_intro);

    // =========================================================================
    // NODE: MARA CONFESSION SOFT (comfort path result)
    // =========================================================================
    let mut mara_confession_soft = DialogueNode::new(
        2,
        CharacterId::new(entities::mara()),
        DialogText::inline(
            "*sighs* Yes. The refugee camp beyond the east wall. Three children with fever. Their parents begged me... I couldn't just watch them die.",
        ),
    );
    mara_confession_soft.outcomes_on_enter = vec![
        set_fact_world(facts::talked_to_mara()),
        set_fact_world(facts::mara_revealed_diversion()),
        set_fact_world(facts::medicine_diverted_to_refugees()),
    ];

    let mut opt_d = DialogueOption::new(
        4,
        DialogText::inline(
            "[Paragon] I'll speak carefully. The truth matters, but so do the people you saved.",
        ),
        DialogTarget::Node(node_mara_promise_choice()),
        node_mara_confession_soft(),
    );
    opt_d.tags = vec![
        tag(Tag::Moral(Moral::Paragon)),
        tag(Tag::SpeechAct(SpeechAct::Promise)),
        tag(Tag::Social(Social::Confidential)),
    ];
    opt_d.intentions = DialogIntentions::new()
        .with_explicit(Intention::Custom("KeepPromise".into()))
        .with_implicit(Intention::Comfort);
    opt_d.outcomes = vec![
        add_relationship(entities::player(), entities::mara(), 10),
        set_fact_world(facts::player_promised_mara_discretion()),
    ];

    let mut opt_e = DialogueOption::new(
        5,
        DialogText::inline("[Truth] I won't hide this. Orren needs to know what happened."),
        DialogTarget::Node(node_mara_promise_choice()),
        node_mara_confession_soft(),
    );
    opt_e.tags = vec![
        tag(Tag::Moral(Moral::Pragmatic)),
        tag(Tag::SpeechAct(SpeechAct::Truth)),
        tag(Tag::Social(Social::Public)),
    ];
    opt_e.intentions = DialogIntentions::new()
        .with_explicit(Intention::Reveal(facts::medicine_diverted_to_refugees()));
    opt_e.outcomes = vec![
        add_relationship(entities::player(), entities::mara(), -5),
        set_fact_world(facts::player_refused_discretion()),
    ];

    mara_confession_soft.options.push(opt_d);
    mara_confession_soft.options.push(opt_e);
    graph.nodes.push(mara_confession_soft);

    // =========================================================================
    // NODE: MARA CONFESSION GUARDED (investigator path result)
    // =========================================================================
    let mut mara_confession_guarded = DialogueNode::new(
        3,
        CharacterId::new(entities::mara()),
        DialogText::inline(
            "The ledgers... *clenches jaw* The Civic Wardens would have let those children die for paperwork. I made a choice. The crates are at the east wall camp.",
        ),
    );
    mara_confession_guarded.outcomes_on_enter = vec![
        set_fact_world(facts::talked_to_mara()),
        set_fact_world(facts::mara_revealed_diversion()),
        set_fact_world(facts::medicine_diverted_to_refugees()),
    ];

    let opt_guarded_continue = DialogueOption::new(
        6,
        DialogText::inline("[Continue] ..."),
        DialogTarget::Node(node_mara_promise_choice()),
        node_mara_confession_guarded(),
    );
    mara_confession_guarded.options.push(opt_guarded_continue);
    graph.nodes.push(mara_confession_guarded);

    // =========================================================================
    // NODE: MARA CONFESSION FORCED (intimidation path result)
    // =========================================================================
    let mut mara_confession_forced = DialogueNode::new(
        4,
        CharacterId::new(entities::mara()),
        DialogText::inline(
            "*voice trembling* The east wall. The refugee camp. Take what you want, just... leave me be.",
        ),
    );
    mara_confession_forced.outcomes_on_enter = vec![
        set_fact_world(facts::talked_to_mara()),
        set_fact_world(facts::mara_revealed_diversion()),
        set_fact_world(facts::medicine_diverted_to_refugees()),
    ];

    let opt_forced_continue = DialogueOption::new(
        7,
        DialogText::inline("[Leave] ..."),
        DialogTarget::Node(node_mara_promise_choice()),
        node_mara_confession_forced(),
    );
    mara_confession_forced.options.push(opt_forced_continue);
    graph.nodes.push(mara_confession_forced);

    // =========================================================================
    // NODE: MARA PROMISE CHOICE (transition to Orren)
    // =========================================================================
    let mut mara_promise = DialogueNode::new(
        5,
        CharacterId::new(entities::mara()),
        DialogText::inline(
            "Whatever happens... thank you for hearing me out. Captain Vale is waiting in the council room.",
        ),
    );

    let opt_go_to_orren = DialogueOption::new(
        8,
        DialogText::inline("[Go to Captain Vale] ..."),
        DialogTarget::Node(node_orren_intro()),
        node_mara_promise_choice(),
    );
    mara_promise.options.push(opt_go_to_orren);
    graph.nodes.push(mara_promise);

    // =========================================================================
    // NODE: MARA REFUSED (aggressive accusation result)
    // =========================================================================
    // Mara refuses to give any information - player learned nothing
    let mut mara_refused = DialogueNode::new(
        6,
        CharacterId::new(entities::mara()),
        DialogText::inline(
            "*goes pale, then stiffens* I have nothing to say to you. If you have accusations, take them to Captain Vale. I'm done here.",
        ),
    );
    mara_refused.outcomes_on_enter = vec![
        set_fact_world(facts::talked_to_mara()),
        set_fact_world(facts::mara_refused_to_talk()),
        set_fact_world(facts::player_learned_nothing()),
        // No diversion fact set - player doesn't know what happened!
    ];

    let opt_go_to_orren_no_info = DialogueOption::new(
        21,
        DialogText::inline("[Leave] Fine. I'll talk to Vale directly."),
        DialogTarget::Node(node_orren_intro()),
        node_mara_refused(),
    );
    mara_refused.options.push(opt_go_to_orren_no_info);
    graph.nodes.push(mara_refused);

    // =========================================================================
    // SCENE 2: ORREN INTRO
    // =========================================================================
    let mut orren_intro = DialogueNode::new(
        10,
        CharacterId::new(entities::orren()),
        DialogText::inline(
            "You spoke to Mara Vey. Good. Then you know whether my missing medicine is a theft, a clerical error, or something worse.",
        ),
    );
    orren_intro.conditions = vec![has_fact(facts::talked_to_mara())];

    // Option F: Tell the Truth
    let mut opt_f = DialogueOption::new(
        10,
        DialogText::inline(
            "[Truth] Mara diverted the medicine to refugees outside the wall. She saved three children with fever.",
        ),
        DialogTarget::Node(node_orren_truth_response()),
        node_orren_intro(),
    );
    opt_f.tags = vec![
        tag(Tag::Moral(Moral::Paragon)),
        tag(Tag::Skill(Skill::Medic)),
        tag(Tag::SpeechAct(SpeechAct::Truth)),
        tag(Tag::Social(Social::FactionLoyalty)),
    ];
    opt_f.intentions = DialogIntentions::new()
        .with_explicit(Intention::Reveal(facts::medicine_diverted_to_refugees()))
        .with_explicit(Intention::Custom("ProtectVulnerable".into()))
        .with_implicit(Intention::Custom("GainOrrenRespect".into()))
        .with_implicit(Intention::Custom("FrameCrimeAsMercy".into()))
        .with_implicit(Intention::Custom("PreserveMaraFromHarshPunishment".into()));
    opt_f.conditions = vec![
        has_fact(facts::talked_to_mara()),
        has_fact(facts::medicine_diverted_to_refugees()),
    ];
    opt_f.outcomes = vec![
        add_relationship(entities::player(), entities::orren(), 10),
        add_standing(entities::orren(), 10),
        set_fact_world(facts::player_told_orren_truth()),
        set_fact_world(facts::orren_respects_player_honesty()),
        set_fact_world(facts::orren_softens_report()),
    ];

    // Option G: Blatant Lie (gets caught if Orren overheard)
    // Only available if you actually know what happened
    let mut opt_g = DialogueOption::new(
        11,
        DialogText::inline(
            "[Lie] Mara found the crates already missing. She had nothing to do with it.",
        ),
        DialogTarget::Node(node_orren_lie_response()),
        node_orren_intro(),
    );
    opt_g.tags = vec![
        tag(Tag::Moral(Moral::Renegade)),
        tag(Tag::Skill(Skill::Streetwise)),
        tag(Tag::SpeechAct(SpeechAct::Lie)),
        tag(Tag::Social(Social::Confidential)),
    ];
    opt_g.intentions = DialogIntentions::new()
        .with_explicit(Intention::Deceive)
        .with_implicit(Intention::Custom("ProtectMara".into()));
    opt_g.conditions = vec![has_fact(facts::medicine_diverted_to_refugees())];
    opt_g.outcomes = vec![
        add_relationship(entities::player(), entities::orren(), -20),
        add_standing(entities::orren(), -15),
        set_fact_world(facts::player_lied_to_orren()),
    ];

    // Option G2: Half-Truth / Deceptive Omission
    // Orren KNOWS this is a lie (he overheard), but he pretends to believe it
    // Only available if you actually know what happened
    let mut opt_g2 = DialogueOption::new(
        16,
        DialogText::inline(
            "[Half-Truth] Mara did find the crates missing... after signing for them. She was trying to help someone.",
        ),
        DialogTarget::Node(node_orren_pretends_belief()),
        node_orren_intro(),
    );
    opt_g2.tags = vec![
        tag(Tag::Moral(Moral::Pragmatic)),
        tag(Tag::Skill(Skill::Streetwise)),
        tag(Tag::SpeechAct(SpeechAct::Lie)), // Still technically a lie by omission
        tag(Tag::Social(Social::Confidential)),
    ];
    opt_g2.intentions = DialogIntentions::new()
        .with_explicit(Intention::Custom("DeceiveByOmission".into()))
        .with_implicit(Intention::Custom("ProtectMara".into()))
        .with_implicit(Intention::Custom("TestOrrensGullibility".into()));
    opt_g2.conditions = vec![has_fact(facts::medicine_diverted_to_refugees())];
    opt_g2.outcomes = vec![
        // Small standing hit but relationship appears stable...
        add_standing(entities::orren(), -5),
        set_fact_world(facts::player_gave_half_truth_to_orren()),
        // Note: Orren will secretly mark player as manipulative via hidden observation handler
    ];

    // Option G3: Bluff (when you know nothing)
    // Available only when Mara refused to talk - you're making things up blindly
    let mut opt_g3 = DialogueOption::new(
        22,
        DialogText::inline(
            "[Bluff] She clammed up when I pressed her. My guess? Bandits hit the shipment before it reached the clinic.",
        ),
        DialogTarget::Node(node_orren_lie_response()),
        node_orren_intro(),
    );
    opt_g3.tags = vec![
        tag(Tag::Moral(Moral::Renegade)),
        tag(Tag::Skill(Skill::Streetwise)),
        tag(Tag::SpeechAct(SpeechAct::Lie)),
    ];
    opt_g3.intentions = DialogIntentions::new()
        .with_explicit(Intention::Deceive)
        .with_implicit(Intention::Custom("BlindGuess".into()));
    opt_g3.conditions = vec![
        has_fact(facts::mara_refused_to_talk()),
        // NOT has_fact(medicine_diverted) - that's the point!
    ];
    opt_g3.outcomes = vec![
        add_relationship(entities::player(), entities::orren(), -25), // Worse penalty for blind bluff
        add_standing(entities::orren(), -20),
        set_fact_world(facts::player_lied_to_orren()),
    ];

    // Option H: Refuse to Answer
    let mut opt_h = DialogueOption::new(
        12,
        DialogText::inline(
            "[Deflect] I gave Mara my word. Ask me what I saw, not what she trusted me with.",
        ),
        DialogTarget::Node(node_orren_refuse_response()),
        node_orren_intro(),
    );
    opt_h.tags = vec![
        tag(Tag::Moral(Moral::Paragon)),
        tag(Tag::SpeechAct(SpeechAct::Deflect)),
        tag(Tag::Social(Social::Confidential)),
    ];
    opt_h.intentions = DialogIntentions::new()
        .with_explicit(Intention::Custom("KeepPromise".into()))
        .with_implicit(Intention::Custom("EarnRespect".into()));
    opt_h.conditions = vec![has_fact(facts::player_promised_mara_discretion())];
    opt_h.outcomes = vec![
        add_relationship(entities::player(), entities::orren(), 5),
        add_relationship(entities::player(), entities::mara(), 5),
        set_fact_world(facts::player_refused_to_betray_mara()),
    ];

    orren_intro.options.push(opt_f);
    orren_intro.options.push(opt_g);
    orren_intro.options.push(opt_g2);
    orren_intro.options.push(opt_g3);
    orren_intro.options.push(opt_h);
    graph.nodes.push(orren_intro);

    // =========================================================================
    // NODE: ORREN TRUTH RESPONSE
    // =========================================================================
    let mut orren_truth = DialogueNode::new(
        11,
        CharacterId::new(entities::orren()),
        DialogText::inline(
            "*nods slowly* I had my suspicions. Mercy is a virtue, but so is order. I'll... soften my report. The Free Clinic will make restitution. Mara keeps her position.",
        ),
    );
    orren_truth.outcomes_on_enter = vec![
        add_relationship(entities::player(), entities::orren(), 5),
        add_standing(entities::orren(), 5),
    ];

    let opt_truth_end = DialogueOption::new(
        13,
        DialogText::inline("[End] Thank you, Captain."),
        DialogTarget::End,
        node_orren_truth_response(),
    );
    orren_truth.options.push(opt_truth_end);
    graph.nodes.push(orren_truth);

    // =========================================================================
    // NODE: ORREN LIE RESPONSE
    // =========================================================================
    let orren_knows = "*eyes narrow* I see. Then perhaps you can explain why I overheard her confession not an hour ago. You protect a thief and lie to my face?";
    let _orren_unaware =
        "*sighs* Very well. I'll note your testimony. But if I find you've misled me...";

    let mut orren_lie = DialogueNode::new(
        12,
        CharacterId::new(entities::orren()),
        DialogText::inline(orren_knows), // Will be dynamically updated based on hidden knowledge
    );

    let opt_lie_end = DialogueOption::new(
        14,
        DialogText::inline("[End] ..."),
        DialogTarget::End,
        node_orren_lie_response(),
    );
    orren_lie.options.push(opt_lie_end);
    graph.nodes.push(orren_lie);

    // =========================================================================
    // NODE: ORREN PRETENDS BELIEF (half-truth response)
    // =========================================================================
    // Orren knows the player is lying (he overheard!), but he pretends to believe
    // This creates dramatic irony - the player thinks they got away with it
    let mut orren_pretends = DialogueNode::new(
        14,
        CharacterId::new(entities::orren()),
        DialogText::inline(
            "*nods, expression unreadable* I appreciate your candor. These are difficult times, and difficult choices must be made. I'll... look into the matter personally.",
        ),
    );
    // Orren appears to accept the story, but secretly he's now watching the player
    orren_pretends.outcomes_on_enter = vec![
        // He acts cordial - small trust gain on surface
        add_relationship(entities::player(), entities::orren(), 5),
        // But standing takes a hidden hit (he's marking you)
        add_standing(entities::orren(), -10),
        set_fact_world(facts::orren_pretends_to_believe()),
        set_fact_world(facts::orren_plotting_to_test_player()),
        // Secretly he knows you're manipulative - hidden fact
        set_hidden_fact(hidden_facts::orren_caught_player_lie()),
    ];

    let opt_pretend_end = DialogueOption::new(
        17,
        DialogText::inline("[End] Thank you, Captain. I knew you'd understand."),
        DialogTarget::End,
        node_orren_pretends_belief(),
    );
    orren_pretends.options.push(opt_pretend_end);
    graph.nodes.push(orren_pretends);

    // =========================================================================
    // NODE: ORREN REFUSE RESPONSE
    // =========================================================================
    let mut orren_refuse = DialogueNode::new(
        13,
        CharacterId::new(entities::orren()),
        DialogText::inline(
            "*studies you for a long moment* A person who keeps their word, even when it's inconvenient. That's rare in Greyford. I won't press you further.",
        ),
    );
    orren_refuse.outcomes_on_enter =
        vec![add_relationship(entities::player(), entities::orren(), 5)];

    let opt_refuse_end = DialogueOption::new(
        15,
        DialogText::inline("[End] Thank you for understanding, Captain."),
        DialogTarget::End,
        node_orren_refuse_response(),
    );
    orren_refuse.options.push(opt_refuse_end);
    graph.nodes.push(orren_refuse);

    graph
}

// =============================================================================
// TUI FUNCTIONS
// =============================================================================

fn clear_screen() {
    print!("\x1B[2J\x1B[1;1H");
    io::stdout().flush().unwrap();
}

fn render_scene(state: &GameState) {
    clear_screen();

    // Header
    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║                 THE BROKEN SEAL - Dramaturge Demo                ║");
    println!("╠══════════════════════════════════════════════════════════════════╣");

    // Speaker and dialogue
    let node = match state.current_node() {
        Some(n) => n,
        None => {
            println!("║  Error: Node not found                                           ║");
            println!("╚══════════════════════════════════════════════════════════════════╝");
            return;
        }
    };

    let text = match &node.text {
        DialogText::Inline(s) => s.as_str(),
        DialogText::LocalizedKey(_) => "<localized>",
    };

    // Dynamic text for Orren's lie response based on hidden knowledge
    let display_text = if state.current_node == node_orren_lie_response() {
        let orren_knows = state.world.fact_exists(
            &FactScope::Character(CharacterId::new(entities::orren())),
            &hidden_facts::orren_caught_player_lie(),
        );
        let player_bluffing = state
            .world
            .fact_exists(&FactScope::World, &facts::mara_refused_to_talk())
            && !state
                .world
                .fact_exists(&FactScope::World, &facts::medicine_diverted_to_refugees());

        if player_bluffing {
            // Player is making wild guesses with no information
            "*laughs coldly* Bandits? Is that the best you can conjure? I had hoped for something more... creative. You've wasted my time."
        } else if orren_knows {
            "*eyes narrow* I see. Then perhaps you can explain why I overheard her confession not an hour ago. You protect a thief and lie to my face?"
        } else {
            "*sighs* Very well. I'll note your testimony. But if I find you've misled me..."
        }
    } else {
        text
    };

    println!("║  {:<64} ║", format!("{}:", state.current_speaker));

    // Word wrap the dialogue text
    let wrapped = word_wrap(display_text, 64);
    for line in &wrapped {
        println!("║  {:<64} ║", line);
    }

    println!("╠══════════════════════════════════════════════════════════════════╣");

    // Available options
    let options = state.available_options();
    if options.is_empty() {
        println!("║  No available options.                                           ║");
    } else {
        for (i, opt) in options.iter().enumerate() {
            let opt_text = match &opt.text {
                DialogText::Inline(s) => s.as_str(),
                DialogText::LocalizedKey(_) => "<localized>",
            };

            // Show tags inline with option
            let tag_str: Vec<String> = opt.tags.iter().map(|t| format!("[{}]", t.key)).collect();
            let tags = tag_str.join("");

            println!("║  {}. {} ", i + 1, opt_text);
            if !tags.is_empty() {
                println!("║     Tags: {:<54}", format!("{}", tags));
            }
        }
    }

    println!("╠══════════════════════════════════════════════════════════════════╣");

    // Status bar
    let mara_trust = state
        .world
        .get_relationship(
            entities::player(),
            entities::mara(),
            &relationships::trust(),
        )
        .unwrap_or(0);
    let orren_trust = state
        .world
        .get_relationship(
            entities::player(),
            entities::orren(),
            &relationships::trust(),
        )
        .unwrap_or(0);

    println!(
        "║  Trust: Mara={:>3}  Orren={:>3}  |  Press 'd' for debug mode        ║",
        mara_trust, orren_trust
    );

    if state.debug_mode {
        println!("╠══════════════════════════════════════════════════════════════════╣");
        println!("║  DEBUG MODE - Hidden Facts (Orren's Knowledge):                  ║");

        let hidden = vec![
            (
                "Overheard Mara",
                hidden_facts::orren_overheard_mara_conversation(),
            ),
            (
                "Knows Diversion",
                hidden_facts::orren_knows_medicine_was_diverted(),
            ),
            (
                "Knows Promise",
                hidden_facts::orren_knows_player_promised_discretion(),
            ),
            (
                "Knows Threat",
                hidden_facts::orren_knows_player_threatened_mara(),
            ),
            ("Caught Lie", hidden_facts::orren_caught_player_lie()),
        ];

        for (name, fact_id) in hidden {
            let known = state.world.fact_exists(
                &FactScope::Character(CharacterId::new(entities::orren())),
                &fact_id,
            );
            let status = if known { "[KNOWN]" } else { "[unknown]" };
            println!("║    {:<20} {:<40} ║", name, status);
        }

        println!("║                                                                  ║");
        println!("║  Visible Facts:                                                  ║");

        let visible = vec![
            ("Talked to Mara", facts::talked_to_mara()),
            ("Mara Revealed", facts::mara_revealed_diversion()),
            ("Diverted Meds", facts::medicine_diverted_to_refugees()),
            (
                "Promised Discretion",
                facts::player_promised_mara_discretion(),
            ),
            ("Threatened Mara", facts::player_threatened_mara()),
            ("Told Truth to Orren", facts::player_told_orren_truth()),
            ("Lied to Orren", facts::player_lied_to_orren()),
            (
                "Half-Truth to Orren",
                facts::player_gave_half_truth_to_orren(),
            ),
            ("Orren Pretends", facts::orren_pretends_to_believe()),
            (
                "Orren Plotting Test",
                facts::orren_plotting_to_test_player(),
            ),
        ];

        for (name, fact_id) in visible {
            let known = state.world.fact_exists(&FactScope::World, &fact_id);
            let status = if known { "[TRUE]" } else { "[false]" };
            println!("║    {:<20} {:<40} ║", name, status);
        }
    }

    println!("╚══════════════════════════════════════════════════════════════════╝");
    print!("\nEnter choice (number): ");
    io::stdout().flush().unwrap();
}

fn word_wrap(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current_line = String::new();

    for word in text.split_whitespace() {
        if current_line.len() + word.len() + 1 > width {
            if !current_line.is_empty() {
                lines.push(current_line.clone());
                current_line.clear();
            }
            current_line.push_str(word);
        } else {
            if !current_line.is_empty() {
                current_line.push(' ');
            }
            current_line.push_str(word);
        }
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }

    lines
}

fn read_line() -> String {
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    input.trim().to_string()
}

fn handle_input(state: &mut GameState, input: &str) -> bool {
    if input.eq_ignore_ascii_case("d") {
        state.debug_mode = !state.debug_mode;
        return true;
    }

    if input.eq_ignore_ascii_case("q") {
        println!("\nExiting...");
        std::process::exit(0);
    }

    match input.parse::<usize>() {
        Ok(n) if n > 0 => {
            state.apply_option(n - 1);
            true
        }
        _ => {
            println!("Invalid input. Please enter a number.");
            false
        }
    }
}

// =============================================================================
// MAIN
// =============================================================================

fn main() {
    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║                                                                  ║");
    println!("║              THE BROKEN SEAL - Dramaturge Example                ║");
    println!("║                                                                  ║");
    println!("║  A dialogue system demonstration featuring:                      ║");
    println!("║  • Typed tags (Moral, Skill, SpeechAct, Social)                  ║");
    println!("║  • Explicit and implicit intentions                              ║");
    println!("║  • Fact-based and relationship conditions                        ║");
    println!("║  • State-changing outcomes                                       ║");
    println!("║  • Hidden observation mechanics                                  ║");
    println!("║                                                                  ║");
    println!("║  Press 'd' during gameplay to toggle debug mode.                 ║");
    println!("║  Press 'q' to quit.                                              ║");
    println!("║                                                                  ║");
    println!("╚══════════════════════════════════════════════════════════════════╝");
    println!();
    println!("Press Enter to begin...");
    read_line();

    let mut state = GameState::new();

    loop {
        render_scene(&state);
        let input = read_line();
        handle_input(&mut state, &input);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tag_axis_and_key() {
        let tag = Tag::Moral(Moral::Paragon);
        assert_eq!(tag.axis(), "moral");
        assert_eq!(tag.key(), "paragon");

        let tag = Tag::Skill(Skill::Medic);
        assert_eq!(tag.axis(), "skill");
        assert_eq!(tag.key(), "medic");

        let tag = Tag::SpeechAct(SpeechAct::Comfort);
        assert_eq!(tag.axis(), "speech");
        assert_eq!(tag.key(), "comfort");

        let tag = Tag::Social(Social::Confidential);
        assert_eq!(tag.axis(), "social");
        assert_eq!(tag.key(), "confidential");
    }

    #[test]
    fn test_tag_conversion() {
        let tag: DialogTagKind = Tag::Moral(Moral::Paragon).into();
        assert_eq!(tag.axis.as_ref(), "moral");
        assert_eq!(tag.key.as_ref(), "paragon");
    }

    #[test]
    fn test_game_state_initialization() {
        let state = GameState::new();
        assert_eq!(state.current_node, node_mara_intro());
        assert!(!state.debug_mode);

        let mara_trust = state.world.get_relationship(
            entities::player(),
            entities::mara(),
            &relationships::trust(),
        );
        assert_eq!(mara_trust, Some(0));
    }

    #[test]
    fn test_available_options_at_start() {
        let state = GameState::new();
        let options = state.available_options();
        assert_eq!(options.len(), 4); // Four initial options with Mara (including aggressive refusal)
    }

    #[test]
    fn test_dialogue_graph_construction() {
        let graph = build_dialogue_graph();
        assert!(!graph.nodes.is_empty());

        // Check that key nodes exist
        let has_mara_intro = graph.nodes.iter().any(|n| n.id == node_mara_intro());
        let has_orren_intro = graph.nodes.iter().any(|n| n.id == node_orren_intro());

        assert!(has_mara_intro);
        assert!(has_orren_intro);
    }

    #[test]
    fn test_hidden_facts_are_character_scoped() {
        let fact_id = hidden_facts::orren_overheard_mara_conversation();
        let scope = FactScope::Character(CharacterId::new(entities::orren()));

        let mut state = GameState::new();
        assert!(!state.world.fact_exists(&scope, &fact_id));

        // Set the hidden fact
        state
            .world
            .set_fact(scope.clone(), Fact::bool(fact_id.0.clone(), true));

        assert!(state.world.fact_exists(&scope, &fact_id));
    }

    #[test]
    fn test_half_truth_node_exists() {
        let graph = build_dialogue_graph();

        // Check that the new "pretends belief" node exists
        let has_pretend_node = graph
            .nodes
            .iter()
            .any(|n| n.id == node_orren_pretends_belief());
        assert!(has_pretend_node, "Orren pretends belief node should exist");

        // Check that Orren intro has 5 options (truth, lie, half-truth, bluff, refuse)
        let orren_intro = graph.nodes.iter().find(|n| n.id == node_orren_intro());
        assert!(orren_intro.is_some());
        assert_eq!(
            orren_intro.unwrap().options.len(),
            5,
            "Orren intro should have 5 options"
        );
    }

    #[test]
    fn test_half_truth_facts() {
        let mut state = GameState::new();

        // Initially, half-truth facts should not exist
        assert!(
            !state
                .world
                .fact_exists(&FactScope::World, &facts::player_gave_half_truth_to_orren())
        );
        assert!(
            !state
                .world
                .fact_exists(&FactScope::World, &facts::orren_pretends_to_believe())
        );
        assert!(
            !state
                .world
                .fact_exists(&FactScope::World, &facts::orren_plotting_to_test_player())
        );

        // Set the facts
        state.world.set_fact(
            FactScope::World,
            Fact::bool(facts::player_gave_half_truth_to_orren().0, true),
        );
        state.world.set_fact(
            FactScope::World,
            Fact::bool(facts::orren_pretends_to_believe().0, true),
        );

        assert!(
            state
                .world
                .fact_exists(&FactScope::World, &facts::player_gave_half_truth_to_orren())
        );
        assert!(
            state
                .world
                .fact_exists(&FactScope::World, &facts::orren_pretends_to_believe())
        );
    }

    #[test]
    fn test_truth_consequences_with_promise() {
        // Test that telling truth after promising discretion damages Mara relationship
        let mut state = GameState::new();

        // Set initial trust values
        state.world.add_relationship(
            entities::player(),
            entities::mara(),
            relationships::trust(),
            20,
        );

        // Player promised discretion
        state.world.set_fact(
            FactScope::World,
            Fact::bool(facts::player_promised_mara_discretion().0, true),
        );

        let initial_mara_trust = state
            .world
            .get_relationship(
                entities::player(),
                entities::mara(),
                &relationships::trust(),
            )
            .unwrap();

        // Simulate telling truth by calling handle_truth_consequences
        state.handle_truth_consequences(&DialogTarget::Node(node_orren_truth_response()));

        let final_mara_trust = state
            .world
            .get_relationship(
                entities::player(),
                entities::mara(),
                &relationships::trust(),
            )
            .unwrap();

        // Should have decreased by 15 for breaking promise
        assert_eq!(final_mara_trust, initial_mara_trust - 15);
    }

    #[test]
    fn test_truth_consequences_with_intimidation() {
        // Test that telling truth after intimidating Mara doesn't further damage relationship
        let mut state = GameState::new();

        // Set initial trust values (low due to intimidation)
        state.world.add_relationship(
            entities::player(),
            entities::mara(),
            relationships::trust(),
            -10,
        );

        // Player intimidated Mara (but didn't promise)
        state.world.set_fact(
            FactScope::World,
            Fact::bool(facts::player_threatened_mara().0, true),
        );

        let initial_mara_trust = state
            .world
            .get_relationship(
                entities::player(),
                entities::mara(),
                &relationships::trust(),
            )
            .unwrap();

        // Simulate telling truth
        state.handle_truth_consequences(&DialogTarget::Node(node_orren_truth_response()));

        let final_mara_trust = state
            .world
            .get_relationship(
                entities::player(),
                entities::mara(),
                &relationships::trust(),
            )
            .unwrap();

        // Should be unchanged (no additional penalty for intimidation path)
        assert_eq!(final_mara_trust, initial_mara_trust);
    }

    #[test]
    fn test_truth_consequences_neutral_path() {
        // Test neutral path (no promise, no intimidation)
        let mut state = GameState::new();

        // Set initial trust values
        state.world.add_relationship(
            entities::player(),
            entities::mara(),
            relationships::trust(),
            10,
        );

        // No promise, no intimidation facts set

        let initial_mara_trust = state
            .world
            .get_relationship(
                entities::player(),
                entities::mara(),
                &relationships::trust(),
            )
            .unwrap();

        // Simulate telling truth
        state.handle_truth_consequences(&DialogTarget::Node(node_orren_truth_response()));

        let final_mara_trust = state
            .world
            .get_relationship(
                entities::player(),
                entities::mara(),
                &relationships::trust(),
            )
            .unwrap();

        // Should increase by 5 for honesty even if it exposes Mara
        assert_eq!(final_mara_trust, initial_mara_trust + 5);
    }

    #[test]
    fn test_mara_refusal_node_exists() {
        let graph = build_dialogue_graph();

        // Check that the refusal node exists
        let has_refusal_node = graph.nodes.iter().any(|n| n.id == node_mara_refused());
        assert!(has_refusal_node, "Mara refusal node should exist");

        // Check that Mara intro has the aggressive accusation option
        let mara_intro = graph.nodes.iter().find(|n| n.id == node_mara_intro());
        assert!(mara_intro.is_some());
        let has_aggressive_option =
            mara_intro.unwrap().options.iter().any(
                |o| matches!(&o.text, DialogText::Inline(s) if s.contains("thief and a liar")),
            );
        assert!(
            has_aggressive_option,
            "Mara intro should have aggressive accusation option"
        );
    }

    #[test]
    fn test_mara_refusal_facts() {
        let mut state = GameState::new();

        // Initially, refusal facts should not exist
        assert!(
            !state
                .world
                .fact_exists(&FactScope::World, &facts::mara_refused_to_talk())
        );
        assert!(
            !state
                .world
                .fact_exists(&FactScope::World, &facts::player_learned_nothing())
        );
        assert!(
            !state
                .world
                .fact_exists(&FactScope::World, &facts::medicine_diverted_to_refugees())
        );

        // Simulate the refusal path
        state.world.set_fact(
            FactScope::World,
            Fact::bool(facts::talked_to_mara().0, true),
        );
        state.world.set_fact(
            FactScope::World,
            Fact::bool(facts::mara_refused_to_talk().0, true),
        );
        state.world.set_fact(
            FactScope::World,
            Fact::bool(facts::player_learned_nothing().0, true),
        );
        // Note: NOT setting medicine_diverted_to_refugees - that's the point!

        assert!(
            state
                .world
                .fact_exists(&FactScope::World, &facts::mara_refused_to_talk())
        );
        assert!(
            state
                .world
                .fact_exists(&FactScope::World, &facts::player_learned_nothing())
        );
        assert!(
            !state
                .world
                .fact_exists(&FactScope::World, &facts::medicine_diverted_to_refugees())
        );

        // Truth option should NOT be available since we don't know about diversion
        let evaluator = DefaultConditionEvaluator;
        let truth_condition = has_fact(facts::medicine_diverted_to_refugees());
        assert!(
            !evaluator.evaluate(&truth_condition, &state.world),
            "Truth option should not be available without learning diversion"
        );

        // But the bluff option SHOULD be available
        let bluff_condition = has_fact(facts::mara_refused_to_talk());
        assert!(
            evaluator.evaluate(&bluff_condition, &state.world),
            "Bluff option should be available when Mara refused to talk"
        );
    }

    #[test]
    fn test_bluff_option_exists() {
        let graph = build_dialogue_graph();

        // Check that the bluff option exists in Orren intro
        let orren_intro = graph.nodes.iter().find(|n| n.id == node_orren_intro());
        assert!(orren_intro.is_some());

        let has_bluff_option = orren_intro
            .unwrap()
            .options
            .iter()
            .any(|o| matches!(&o.text, DialogText::Inline(s) if s.contains("Bandits")));
        assert!(has_bluff_option, "Orren intro should have bluff option");

        // Check that bluff option requires MaraRefusedToTalk but NOT MedicineDivertedToRefugees
        let bluff_option = orren_intro
            .unwrap()
            .options
            .iter()
            .find(|o| matches!(&o.text, DialogText::Inline(s) if s.contains("Bandits")))
            .unwrap();

        // Should have condition for mara_refused_to_talk
        let has_refusal_condition = bluff_option.conditions.iter().any(|c| {
            matches!(c, Condition::FactExists { fact_id, .. } if fact_id.0.as_ref() == "MaraRefusedToTalk")
        });
        assert!(
            has_refusal_condition,
            "Bluff option should require MaraRefusedToTalk"
        );
    }

    #[test]
    fn test_intimidation_path_options_at_orren() {
        // Simulate the intimidation path and verify options are available at Orren
        let mut state = GameState::new();

        // Apply outcomes from mara_confession_forced node (what happens after intimidation)
        state.world.set_fact(
            FactScope::World,
            Fact::bool(facts::talked_to_mara().0, true),
        );
        state.world.set_fact(
            FactScope::World,
            Fact::bool(facts::mara_revealed_diversion().0, true),
        );
        state.world.set_fact(
            FactScope::World,
            Fact::bool(facts::medicine_diverted_to_refugees().0, true),
        );
        state.world.set_fact(
            FactScope::World,
            Fact::bool(facts::player_threatened_mara().0, true),
        );

        // Move to Orren's node
        state.current_node = node_orren_intro();

        // Check available options
        let options = state.available_options();

        // Should have: Truth, Lie, Half-Truth (5 total including Bluff and Deflect)
        // But Deflect requires promise which wasn't made, Bluff requires refusal
        // So should have 3: Truth, Lie, Half-Truth
        assert!(
            options.len() >= 3,
            "Should have at least 3 options at Orren after intimidation, found {}",
            options.len()
        );

        // Verify Truth option is available
        let has_truth = options.iter().any(
            |o| matches!(&o.text, DialogText::Inline(s) if s.contains("diverted the medicine")),
        );
        assert!(has_truth, "Truth option should be available");

        // Verify Lie option is available
        let has_lie = options.iter().any(|o| {
            matches!(&o.text, DialogText::Inline(s) if s.contains("found the crates already missing"))
        });
        assert!(has_lie, "Lie option should be available");
    }
}
