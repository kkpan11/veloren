use super::{
    super::{Animation, vek::*},
    GolemSkeleton, SkeletonAttr,
};
use common::states::utils::StageSection;
use core::f32::consts::PI;

pub struct AlphaAnimation;

impl Animation for AlphaAnimation {
    type Dependency<'a> = (Option<StageSection>, f32, f32, Option<&'a str>);
    type Skeleton = GolemSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"golem_alpha\0";

    #[cfg_attr(feature = "be-dyn-lib", unsafe(export_name = "golem_alpha"))]

    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (stage_section, global_time, timer, ability_id): Self::Dependency<'_>,
        anim_time: f32,
        _rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        match ability_id {
            Some(
                "common.abilities.custom.stonegolemfist.spin"
                | "common.abilities.custom.woodgolem.spin"
                | "common.abilities.custom.coralgolem.spin"
                | "common.abilities.custom.irongolemfist.spin",
            ) => {
                let (movement1, movement2, movement3) = match stage_section {
                    Some(StageSection::Buildup) => (anim_time.powf(0.25), 0.0, 0.0),
                    Some(StageSection::Action) => (1.0, anim_time, 0.0),
                    Some(StageSection::Recover) => (1.0, 1.0, anim_time.powf(4.0)),
                    _ => (0.0, 0.0, 0.0),
                };

                let pullback = 1.0 - movement3;

                next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1) * 1.02;
                next.head.orientation =
                    Quaternion::rotation_z(movement1 * 0.5 * PI + movement2 * -2.5 * PI)
                        * Quaternion::rotation_x(-0.2);

                next.upper_torso.position =
                    Vec3::new(0.0, s_a.upper_torso.0, s_a.upper_torso.1 + movement1 * -6.0);
                next.upper_torso.orientation =
                    Quaternion::rotation_z(movement1 * -0.5 * PI + movement2 * 2.5 * PI);

                next.lower_torso.position = Vec3::new(0.0, s_a.lower_torso.0, s_a.lower_torso.1);
                next.lower_torso.orientation =
                    Quaternion::rotation_z(movement1 * 0.5 * PI + movement2 * -2.5 * PI);

                next.shoulder_l.position =
                    Vec3::new(-s_a.shoulder.0, s_a.shoulder.1, s_a.shoulder.2);
                next.shoulder_l.orientation = Quaternion::rotation_x(0.0)
                    * Quaternion::rotation_x(movement1 * 1.2 * pullback);

                next.shoulder_r.position =
                    Vec3::new(s_a.shoulder.0, s_a.shoulder.1, s_a.shoulder.2);
                next.shoulder_r.orientation = Quaternion::rotation_x(0.0)
                    * Quaternion::rotation_x(movement1 * -1.2 * pullback);

                next.hand_l.position = Vec3::new(-s_a.hand.0, s_a.hand.1, s_a.hand.2);
                next.hand_l.orientation = Quaternion::rotation_x(movement1 * -0.2 * pullback);

                next.hand_r.position = Vec3::new(s_a.hand.0, s_a.hand.1, s_a.hand.2);
                next.hand_r.orientation = Quaternion::rotation_x(movement1 * 0.2 * pullback);

                next.leg_l.position =
                    Vec3::new(-s_a.leg.0, s_a.leg.1, s_a.leg.2 + movement1 * 2.0) * 1.02;
                next.leg_l.orientation = Quaternion::rotation_x(0.0);

                next.leg_r.position =
                    Vec3::new(s_a.leg.0, s_a.leg.1, s_a.leg.2 + movement1 * 2.0) * 1.02;
                next.leg_r.orientation = Quaternion::rotation_x(0.0);

                next.foot_l.position =
                    Vec3::new(-s_a.foot.0, s_a.foot.1, s_a.foot.2 + movement1 * 4.0);
                next.foot_l.orientation = Quaternion::rotation_x(0.0);

                next.foot_r.position =
                    Vec3::new(s_a.foot.0, s_a.foot.1, s_a.foot.2 + movement1 * 4.0);
                next.foot_r.orientation = Quaternion::rotation_x(0.0);

                next.torso.position = Vec3::new(0.0, 0.0, 0.0);
                next.torso.orientation = Quaternion::rotation_z(0.0);
            },
            _ => {
                let (move1base, move2base, move3) = match stage_section {
                    Some(StageSection::Buildup) => (anim_time.powf(0.25), 0.0, 0.0),
                    Some(StageSection::Action) => (1.0, anim_time, 0.0),
                    Some(StageSection::Recover) => (1.0, 1.0, anim_time.powf(4.0)),
                    _ => (0.0, 0.0, 0.0),
                };

                let pullback = 1.0 - move3;
                let subtract = global_time - timer;
                let check = subtract - subtract.trunc();
                let mirror = (check - 0.5).signum();

                let move1 = move1base * pullback;
                let move2 = move2base * pullback;
                if mirror > 0.0 {
                    next.head.orientation = Quaternion::rotation_x(-0.2)
                        * Quaternion::rotation_z(move1 * -1.2 + move2 * 2.0);

                    next.upper_torso.orientation = Quaternion::rotation_x(move1 * -0.6)
                        * Quaternion::rotation_z(move1 * 1.2 + move2 * -3.2);

                    next.lower_torso.orientation =
                        Quaternion::rotation_z(move1 * -1.2 + move2 * 3.2)
                            * Quaternion::rotation_x(move1 * 0.6);

                    next.shoulder_l.orientation = Quaternion::rotation_y(move1 * 0.8)
                        * Quaternion::rotation_x(move1 * -1.0 + move2 * 1.6);

                    next.shoulder_r.orientation = Quaternion::rotation_x(move1 * 0.4);

                    next.hand_l.orientation = Quaternion::rotation_z(0.0)
                        * Quaternion::rotation_x(move1 * -1.0 + move2 * 1.8);

                    next.hand_r.orientation =
                        Quaternion::rotation_y(move1 * 0.5) * Quaternion::rotation_x(move1 * 0.4);
                } else {
                    next.head.orientation = Quaternion::rotation_x(-0.2)
                        * Quaternion::rotation_z(move1 * 1.2 + move2 * -2.0);

                    next.upper_torso.orientation = Quaternion::rotation_x(move1 * -0.6)
                        * Quaternion::rotation_z(move1 * -1.2 + move2 * 3.2);

                    next.lower_torso.orientation =
                        Quaternion::rotation_z(move1 * 1.2 + move2 * -3.2)
                            * Quaternion::rotation_x(move1 * 0.6);

                    next.shoulder_l.orientation = Quaternion::rotation_x(move1 * 0.4);

                    next.shoulder_r.orientation = Quaternion::rotation_y(move1 * -0.8)
                        * Quaternion::rotation_x(move1 * -1.0 + move2 * 1.6);

                    next.hand_l.orientation =
                        Quaternion::rotation_y(move1 * -0.5) * Quaternion::rotation_x(move1 * 0.4);

                    next.hand_r.orientation = Quaternion::rotation_y(0.0)
                        * Quaternion::rotation_x(move1 * -1.0 + move2 * 1.8);
                };
                next.torso.position = Vec3::new(0.0, move1 * 3.7, move1 * -1.6);
            },
        }

        next
    }
}
