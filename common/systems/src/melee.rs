use common::{
    GroupTarget,
    combat::{self, AttackOptions, AttackSource, AttackerInfo, TargetInfo},
    comp::{
        Alignment, Body, Buffs, CharacterState, Combo, Energy, Group, Health, Inventory, Mass,
        Melee, Ori, PhysicsState, Player, Pos, Scale, Stats,
        ability::Dodgeable,
        agent::{Sound, SoundKind},
        aura::EnteredAuras,
        melee::MultiTarget,
    },
    event::{self, EmitExt, EventBus},
    event_emitters,
    outcome::Outcome,
    resources::Time,
    terrain::TerrainGrid,
    uid::{IdMaps, Uid},
    util::{Dir, find_dist::Cylinder},
    vol::ReadVol,
};
use common_ecs::{Job, Origin, Phase, System};
use itertools::Itertools;
use specs::{
    Entities, Join, LendJoin, Read, ReadExpect, ReadStorage, SystemData, WriteStorage, shred,
};
use vek::*;

event_emitters! {
    struct ReadAttackEvents[AttackEmitters] {
        health_change: event::HealthChangeEvent,
        energy_change: event::EnergyChangeEvent,
        poise_change: event::PoiseChangeEvent,
        sound: event::SoundEvent,
        mine_block: event::MineBlockEvent,
        parry_hook: event::ParryHookEvent,
        knockback: event::KnockbackEvent,
        entity_attack_hook: event::EntityAttackedHookEvent,
        combo_change: event::ComboChangeEvent,
        buff: event::BuffEvent,
    }
}

#[derive(SystemData)]
pub struct ReadData<'a> {
    time: Read<'a, Time>,
    terrain: ReadExpect<'a, TerrainGrid>,
    id_maps: Read<'a, IdMaps>,
    entities: Entities<'a>,
    players: ReadStorage<'a, Player>,
    uids: ReadStorage<'a, Uid>,
    positions: ReadStorage<'a, Pos>,
    orientations: ReadStorage<'a, Ori>,
    alignments: ReadStorage<'a, Alignment>,
    scales: ReadStorage<'a, Scale>,
    bodies: ReadStorage<'a, Body>,
    healths: ReadStorage<'a, Health>,
    energies: ReadStorage<'a, Energy>,
    inventories: ReadStorage<'a, Inventory>,
    groups: ReadStorage<'a, Group>,
    char_states: ReadStorage<'a, CharacterState>,
    physic_states: ReadStorage<'a, PhysicsState>,
    stats: ReadStorage<'a, Stats>,
    combos: ReadStorage<'a, Combo>,
    buffs: ReadStorage<'a, Buffs>,
    entered_auras: ReadStorage<'a, EnteredAuras>,
    events: ReadAttackEvents<'a>,
    masses: ReadStorage<'a, Mass>,
}

/// This system is responsible for handling accepted inputs like moving or
/// attacking
#[derive(Default)]
pub struct Sys;

impl<'a> System<'a> for Sys {
    type SystemData = (
        ReadData<'a>,
        WriteStorage<'a, Melee>,
        Read<'a, EventBus<Outcome>>,
    );

    const NAME: &'static str = "melee";
    const ORIGIN: Origin = Origin::Common;
    const PHASE: Phase = Phase::Create;

    fn run(_job: &mut Job<Self>, (read_data, mut melee_attacks, outcomes): Self::SystemData) {
        let mut emitters = read_data.events.get_emitters();
        let mut outcomes_emitter = outcomes.emitter();
        let mut rng = rand::thread_rng();

        // Attacks
        for (attacker, uid, pos, ori, melee_attack, body, scale) in (
            &read_data.entities,
            &read_data.uids,
            &read_data.positions,
            &read_data.orientations,
            &mut melee_attacks,
            &read_data.bodies,
            read_data.scales.maybe(),
        )
            .join()
        {
            if melee_attack.applied {
                continue;
            }
            emitters.emit(event::SoundEvent {
                sound: Sound::new(SoundKind::Melee, pos.0, 2.0, read_data.time.0),
            });
            melee_attack.applied = true;

            // Scales
            let eye_pos = pos.0 + Vec3::unit_z() * body.eye_height(scale.map_or(1.0, |s| s.0));
            let scale = read_data.scales.get(attacker).map_or(1.0, |s| s.0);
            let height = body.height() * scale;
            // TODO: use Capsule Prisms instead of Cylinders
            let rad = body.max_radius() * scale;

            let melee_z = pos.0.z + 0.5 * body.height();
            let melee_z_range = (melee_z - melee_attack.range)..(melee_z + melee_attack.range);

            // Mine blocks broken by the attack
            if let Some((block_pos, tool)) = melee_attack.break_block {
                // Check distance to block
                if eye_pos.distance_squared(block_pos.map(|e| e as f32 + 0.5))
                    < (rad + scale * melee_attack.range).powi(2)
                {
                    emitters.emit(event::MineBlockEvent {
                        entity: attacker,
                        pos: block_pos,
                        tool,
                    });
                }
            }

            // Go through all other entities
            for (target, pos_b, health_b, body_b, uid_b) in (
                &read_data.entities,
                &read_data.positions,
                &read_data.healths,
                &read_data.bodies,
                &read_data.uids,
            )
                .join()
                .sorted_by_key(|(_, pos_b, _, _, _)| pos_b.0.distance_squared(pos.0) as u32)
            {
                // Unless the melee attack can hit multiple targets, stop the attack if it has
                // already hit 1 target
                if melee_attack.multi_target.is_none() && melee_attack.hit_count > 0 {
                    break;
                }

                let look_dir = *ori.look_dir();

                // 2D versions
                let pos2 = Vec2::from(pos.0);
                let pos_b2 = Vec2::<f32>::from(pos_b.0);
                let ori2 = Vec2::from(look_dir);

                // Scales
                let scale_b = read_data.scales.get(target).map_or(1.0, |s| s.0);
                let height_b = body_b.height() * scale_b;
                let rad_b = body_b.max_radius() * scale_b;

                // Check if entity is dodging
                let target_dodging = match melee_attack.dodgeable {
                    Dodgeable::Roll => read_data
                        .char_states
                        .get(target)
                        .and_then(|cs| cs.roll_attack_immunities())
                        .is_some_and(|i| i.melee),
                    Dodgeable::Jump => read_data
                        .physic_states
                        .get(target)
                        .is_some_and(|ps| ps.on_ground.is_none()),
                    Dodgeable::No => false,
                };

                // Check if it is a hit
                let hit = attacker != target
                    && !health_b.is_dead
                    // Spherical wedge shaped attack field
                    && pos.0.distance_squared(pos_b.0) < (rad + rad_b + scale * melee_attack.range).powi(2)
                    && (melee_z_range.contains(&pos_b.0.z) || melee_z_range.contains(&(pos_b.0.z + body_b.height())) || (pos_b.0.z..(pos_b.0.z + body_b.height())).contains(&melee_z))
                    && ori2.angle_between(pos_b2 - pos2) < melee_attack.max_angle + (rad_b / pos2.distance(pos_b2)).atan();

                //Check if target is behind a wall
                let attacker_cylinder = Cylinder {
                    center: pos.0 + (0.5 * height * Vec3::unit_z()),
                    radius: rad,
                    height,
                };
                let target_cylinder = Cylinder {
                    center: pos_b.0 + (0.5 * height_b * Vec3::unit_z()),
                    radius: rad_b,
                    height: height_b,
                };
                let hit = hit
                    && !is_blocked_by_wall(&read_data.terrain, attacker_cylinder, target_cylinder);

                if hit {
                    let allow_friendly_fire =
                        combat::allow_friendly_fire(&read_data.entered_auras, attacker, target);
                    // See if entities are in the same group
                    let same_group = read_data
                        .groups
                        .get(attacker)
                        .map(|group_a| Some(group_a) == read_data.groups.get(target))
                        .unwrap_or(false);

                    let target_group = if same_group {
                        GroupTarget::InGroup
                    } else {
                        GroupTarget::OutOfGroup
                    };

                    let dir = Dir::new((pos_b.0 - pos.0).try_normalized().unwrap_or(look_dir));

                    let attacker_info = Some(AttackerInfo {
                        entity: attacker,
                        uid: *uid,
                        group: read_data.groups.get(attacker),
                        energy: read_data.energies.get(attacker),
                        combo: read_data.combos.get(attacker),
                        inventory: read_data.inventories.get(attacker),
                        stats: read_data.stats.get(attacker),
                        mass: read_data.masses.get(attacker),
                    });

                    let target_ori = read_data.orientations.get(target);
                    let target_char_state = read_data.char_states.get(target);
                    let target_info = TargetInfo {
                        entity: target,
                        uid: *uid_b,
                        inventory: read_data.inventories.get(target),
                        stats: read_data.stats.get(target),
                        health: read_data.healths.get(target),
                        pos: pos_b.0,
                        ori: target_ori,
                        char_state: target_char_state,
                        energy: read_data.energies.get(target),
                        buffs: read_data.buffs.get(target),
                        mass: read_data.masses.get(target),
                    };

                    // PvP check
                    let permit_pvp = combat::permit_pvp(
                        &read_data.alignments,
                        &read_data.players,
                        &read_data.entered_auras,
                        &read_data.id_maps,
                        Some(attacker),
                        target,
                    );

                    // Note: Don't use ori.look_vec() here, it leads to incorrect results for wide
                    // angle melee attacks
                    let precision_from_flank = combat::precision_mult_from_flank(
                        (pos_b.0 - pos.0)
                            .with_z(0.0)
                            .try_normalized()
                            .unwrap_or(ori.look_vec()),
                        target_ori,
                        melee_attack.precision_flank_multipliers,
                        melee_attack.precision_flank_invert,
                    );

                    let precision_from_poise = {
                        if let Some(CharacterState::Stunned(data)) = target_char_state {
                            Some(
                                combat::MAX_MELEE_POISE_PRECISION
                                    * data.static_data.poise_state.damage_multiplier(),
                            )
                        } else {
                            None
                        }
                    };

                    let precision_mult = match (precision_from_flank, precision_from_poise) {
                        (Some(a), Some(b)) => Some(a.max(b)),
                        (Some(a), None) | (None, Some(a)) => Some(a),
                        (None, None) => None,
                    };

                    let attack_options = AttackOptions {
                        target_dodging,
                        permit_pvp,
                        allow_friendly_fire,
                        target_group,
                        precision_mult,
                    };

                    let strength =
                        if let Some(MultiTarget::Scaling(scaling)) = melee_attack.multi_target {
                            1.0 + melee_attack.hit_count as f32 * scaling
                        } else {
                            1.0
                        };

                    let mut is_applied = false;
                    for offset in 0..melee_attack.simultaneous_hits {
                        is_applied = melee_attack.attack.apply_attack(
                            attacker_info,
                            &target_info,
                            dir,
                            attack_options,
                            strength,
                            AttackSource::Melee,
                            *read_data.time,
                            &mut emitters,
                            |o| outcomes_emitter.emit(o),
                            &mut rng,
                            offset as u64,
                        ) || is_applied;
                    }

                    if is_applied {
                        melee_attack.hit_count += melee_attack.simultaneous_hits;
                    }
                }
            }
        }
    }
}

// Cast rays from the corners of an Axis Aligned Box centered at the attacker to
// one centered at the target. We use multiple rays to ensure that target is at
// least almost completly behind a wall.
fn is_blocked_by_wall(terrain: &TerrainGrid, attacker: Cylinder, target: Cylinder) -> bool {
    let attacker_v = Vec3::new(
        attacker.radius / f32::sqrt(2.),
        attacker.radius / f32::sqrt(2.),
        attacker.height / 2.0,
    );
    let attacker_aabb = Aabb {
        min: attacker.center - attacker_v,
        max: attacker.center + attacker_v,
    };

    let target_v = Vec3::new(
        target.radius / f32::sqrt(2.),
        target.radius / f32::sqrt(2.),
        target.height / 2.0,
    );
    let target_aabb = Aabb {
        min: target.center - target_v,
        max: target.center + target_v,
    };

    let mut segments = Vec::with_capacity(9);
    segments.push(LineSegment3 {
        start: attacker.center,
        end: target.center,
    });
    for i in 0..2 {
        for j in 0..2 {
            for l in 0..2 {
                let (x1, x2) = if i == 0 {
                    (attacker_aabb.min.x, target_aabb.min.x)
                } else {
                    (attacker_aabb.max.x, target_aabb.max.x)
                };

                let (y1, y2) = if j == 0 {
                    (attacker_aabb.min.y, target_aabb.min.y)
                } else {
                    (attacker_aabb.max.y, target_aabb.max.y)
                };

                let (z1, z2) = if l == 0 {
                    (attacker_aabb.min.z, target_aabb.min.z)
                } else {
                    (attacker_aabb.max.z, target_aabb.max.z)
                };

                segments.push(LineSegment3 {
                    start: Vec3::new(x1, y1, z1),
                    end: Vec3::new(x2, y2, z2),
                });
            }
        }
    }

    for &segment in segments.iter() {
        let ray_dist = terrain
            .ray(segment.start, segment.end)
            .until(|b| b.is_filled())
            .cast()
            .0;
        if let Some(ray_direction) = (segment.end - segment.start).try_normalized() {
            let ray_end = segment.start + ray_dist * ray_direction;
            let ray = LineSegment3 {
                start: segment.start,
                end: ray_end,
            };
            let proj = ray.projected_point(target_aabb.center());

            if target_aabb.contains_point(proj) {
                return false;
            }
        } else {
            return false;
        }
    }
    true
}
