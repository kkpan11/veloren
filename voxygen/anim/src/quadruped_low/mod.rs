pub mod beta;
pub mod breathe;
pub mod combomelee;
pub mod dash;
pub mod idle;
pub mod jump;
pub mod leapshockwave;
pub mod run;
pub mod shockwave;
pub mod shoot;
pub mod spritesummon;
pub mod stunned;
pub mod tailwhip;

// Reexports
pub use self::{
    beta::BetaAnimation, breathe::BreatheAnimation, combomelee::ComboAnimation,
    dash::DashAnimation, idle::IdleAnimation, jump::JumpAnimation,
    leapshockwave::LeapShockAnimation, run::RunAnimation, shockwave::ShockwaveAnimation,
    shoot::ShootAnimation, spritesummon::SpriteSummonAnimation, stunned::StunnedAnimation,
    tailwhip::TailwhipAnimation,
};

use super::{FigureBoneData, Offsets, Skeleton, make_bone, vek::*};
use common::{
    comp::{self},
    states::utils::StageSection,
};
use core::convert::TryFrom;

pub type Body = comp::quadruped_low::Body;

skeleton_impls!(struct QuadrupedLowSkeleton {
    + head_c_upper,
    + pub head_c_lower,
    + jaw_c,
    + head_l_upper,
    + pub head_l_lower,
    + jaw_l,
    + head_r_upper,
    + pub head_r_lower,
    + jaw_r,
    + chest,
    + tail_front,
    + tail_rear,
    + foot_fl,
    + foot_fr,
    + foot_bl,
    + foot_br,
    mount,
});

impl Skeleton for QuadrupedLowSkeleton {
    type Attr = SkeletonAttr;
    type Body = Body;

    const BONE_COUNT: usize = 16;
    #[cfg(feature = "use-dyn-lib")]
    const COMPUTE_FN: &'static [u8] = b"quadruped_low_compute_mats\0";

    #[cfg_attr(
        feature = "be-dyn-lib",
        unsafe(export_name = "quadruped_low_compute_mats")
    )]
    fn compute_matrices_inner(
        &self,
        base_mat: Mat4<f32>,
        buf: &mut [FigureBoneData; super::MAX_BONE_COUNT],
        body: Self::Body,
    ) -> Offsets {
        let attr = SkeletonAttr::from(&body);
        let base_mat = base_mat * Mat4::scaling_3d(attr.scaler / 11.0);

        let chest_mat = base_mat * Mat4::<f32>::from(self.chest);
        let tail_front = chest_mat * Mat4::<f32>::from(self.tail_front);
        let tail_rear = tail_front * Mat4::<f32>::from(self.tail_rear);
        let head_c_lower_mat = chest_mat * Mat4::<f32>::from(self.head_c_lower);
        let head_c_upper_mat = head_c_lower_mat * Mat4::<f32>::from(self.head_c_upper);
        let head_l_lower_mat = chest_mat * Mat4::<f32>::from(self.head_l_lower);
        let head_l_upper_mat = head_l_lower_mat * Mat4::<f32>::from(self.head_l_upper);
        let head_r_lower_mat = chest_mat * Mat4::<f32>::from(self.head_r_lower);
        let head_r_upper_mat = head_r_lower_mat * Mat4::<f32>::from(self.head_r_upper);

        *(<&mut [_; Self::BONE_COUNT]>::try_from(&mut buf[0..Self::BONE_COUNT]).unwrap()) = [
            make_bone(head_c_upper_mat),
            make_bone(head_c_lower_mat),
            make_bone(head_c_upper_mat * Mat4::<f32>::from(self.jaw_c)),
            make_bone(head_l_upper_mat),
            make_bone(head_l_lower_mat),
            make_bone(head_l_upper_mat * Mat4::<f32>::from(self.jaw_l)),
            make_bone(head_r_upper_mat),
            make_bone(head_r_lower_mat),
            make_bone(head_r_upper_mat * Mat4::<f32>::from(self.jaw_r)),
            make_bone(chest_mat),
            make_bone(tail_front),
            make_bone(tail_rear),
            make_bone(chest_mat * Mat4::<f32>::from(self.foot_fl)),
            make_bone(chest_mat * Mat4::<f32>::from(self.foot_fr)),
            make_bone(chest_mat * Mat4::<f32>::from(self.foot_bl)),
            make_bone(chest_mat * Mat4::<f32>::from(self.foot_br)),
        ];
        //let (mount_bone_mat, mount_bone_ori) = (chest_mat, self.chest.orientation);
        // Offset from the mounted bone's origin.
        // Note: This could be its own bone if we need to animate it independently.

        // NOTE: We apply the ori from base_mat externally so we don't need to worry
        // about it here for now.

        use comp::quadruped_low::Species::*;
        let (mount_bone_mat, mount_bone_ori) = match (body.species, body.body_type) {
            (Maneater, _) => (
                head_c_upper_mat,
                self.chest.orientation
                    * self.head_c_lower.orientation
                    * self.head_c_upper.orientation,
            ),
            _ => (chest_mat, self.chest.orientation),
        };
        let mount_position = (mount_bone_mat * Vec4::from_point(mount_point(&body)))
            .homogenized()
            .xyz();
        let mount_orientation = mount_bone_ori;

        Offsets {
            viewpoint: Some((head_c_upper_mat * Vec4::new(0.0, 4.0, 1.0, 1.0)).xyz()),
            mount_bone: Transform {
                position: mount_position,
                orientation: mount_orientation,
                scale: Vec3::one(),
            },
            heads: vec![
                (head_l_upper_mat * Vec4::unit_w()).xyz(),
                (head_c_upper_mat * Vec4::unit_w()).xyz(),
                (head_r_upper_mat * Vec4::unit_w()).xyz(),
            ],
            tail: Some((
                (tail_front * Vec4::unit_w()).xyz(),
                (tail_rear * Vec4::new(0.0, -attr.tail_rear_length, 0.0, 1.0)).xyz(),
            )),
            ..Default::default()
        }
    }
}

pub struct SkeletonAttr {
    head_upper: (f32, f32),
    head_lower: (f32, f32),
    jaw: (f32, f32),
    side_head_lower: (f32, f32, f32),
    side_head_upper: (f32, f32, f32),
    chest: (f32, f32),
    tail_front: (f32, f32),
    tail_rear: (f32, f32),
    tail_rear_length: f32,
    feet_f: (f32, f32, f32),
    feet_b: (f32, f32, f32),
    lean: (f32, f32),
    scaler: f32,
    tempo: f32,
    tongue_for_tail: bool,
}

impl<'a> TryFrom<&'a comp::Body> for SkeletonAttr {
    type Error = ();

    fn try_from(body: &'a comp::Body) -> Result<Self, Self::Error> {
        match body {
            comp::Body::QuadrupedLow(body) => Ok(SkeletonAttr::from(body)),
            _ => Err(()),
        }
    }
}

impl Default for SkeletonAttr {
    fn default() -> Self {
        Self {
            head_upper: (0.0, 0.0),
            head_lower: (0.0, 0.0),
            jaw: (0.0, 0.0),
            side_head_lower: (0.0, 0.0, 0.0),
            side_head_upper: (0.0, 0.0, 0.0),
            chest: (0.0, 0.0),
            tail_front: (0.0, 0.0),
            tail_rear: (0.0, 0.0),
            tail_rear_length: 0.0,
            feet_f: (0.0, 0.0, 0.0),
            feet_b: (0.0, 0.0, 0.0),
            lean: (0.0, 0.0),
            scaler: 0.0,
            tempo: 0.0,
            tongue_for_tail: false,
        }
    }
}

impl<'a> From<&'a Body> for SkeletonAttr {
    fn from(body: &'a Body) -> Self {
        use comp::quadruped_low::Species::*;
        Self {
            head_upper: match (body.species, body.body_type) {
                (Crocodile, _) => (1.5, 2.0),
                (SeaCrocodile, _) => (1.5, 2.0),
                (Alligator, _) => (0.5, 2.0),
                (Salamander, _) => (0.5, 1.0),
                (Elbst, _) => (0.5, 1.0),
                (Monitor, _) => (5.5, 3.0),
                (Asp, _) => (4.5, 10.5),
                (Tortoise, _) => (5.0, 1.0),
                (Rocksnapper, _) => (6.0, 0.5),
                (Rootsnapper, _) => (6.0, 0.5),
                (Reefsnapper, _) => (6.0, 0.5),
                (Pangolin, _) => (-0.5, 8.0),
                (Maneater, _) => (7.0, 11.5),
                (Sandshark, _) => (8.5, 0.5),
                (Hakulaq, _) => (8.0, 10.0),
                (Dagon, _) => (8.0, 10.0),
                (Lavadrake, _) => (7.0, 8.0),
                (Icedrake, _) => (7.0, 8.0),
                (Basilisk, _) => (5.0, 2.5),
                (Deadwood, _) => (2.0, -3.0),
                (Mossdrake, _) => (7.0, 8.0),
                (Driggle, _) => (3.0, 4.0),
                (Snaretongue, _) => (7.0, 5.5),
                (Hydra, _) => (12.0, 19.0),
            },
            head_lower: match (body.species, body.body_type) {
                (Crocodile, _) => (8.0, 0.0),
                (SeaCrocodile, _) => (8.0, 0.0),
                (Alligator, _) => (9.0, 0.25),
                (Salamander, _) => (9.0, 0.0),
                (Elbst, _) => (9.0, 0.0),
                (Monitor, _) => (7.0, 0.0),
                (Asp, _) => (6.0, -2.5),
                (Tortoise, _) => (12.0, -3.5),
                (Rocksnapper, _) => (12.0, -9.0),
                (Rootsnapper, _) => (12.0, -9.0),
                (Reefsnapper, _) => (12.0, -9.0),
                (Pangolin, _) => (8.0, -9.0),
                (Maneater, _) => (1.0, 4.5),
                (Sandshark, _) => (13.5, -10.5),
                (Hakulaq, _) => (10.5, 1.0),
                (Dagon, _) => (12.0, -6.0),
                (Lavadrake, _) => (9.0, -6.0),
                (Icedrake, _) => (11.5, -6.0),
                (Basilisk, _) => (12.5, -5.5),
                (Deadwood, _) => (0.0, 0.0),
                (Mossdrake, _) => (9.0, -6.0),
                (Driggle, _) => (6.0, -3.0),
                (Snaretongue, _) => (8.5, 0.0),
                (Hydra, _) => (8.0, -6.5),
            },
            side_head_lower: match (body.species, body.body_type) {
                (Hydra, _) => (9.0, 10.0, -6.5),
                _ => (0.0, 0.0, 0.0),
            },
            side_head_upper: match (body.species, body.body_type) {
                (Hydra, _) => ((1.0), (7.0), (17.0)),
                _ => (0.0, 0.0, 0.0),
            },
            jaw: match (body.species, body.body_type) {
                (Crocodile, _) => (2.5, -3.0),
                (SeaCrocodile, _) => (2.5, -3.0),
                (Alligator, _) => (2.5, -2.0),
                (Salamander, _) => (0.5, -1.0),
                (Elbst, _) => (0.5, -1.0),
                (Monitor, _) => (3.0, -1.0),
                (Asp, _) => (2.0, -2.0),
                (Tortoise, _) => (-3.5, -2.0),
                (Rocksnapper, _) => (-5.0, -1.5),
                (Rootsnapper, _) => (-5.0, -1.5),
                (Reefsnapper, _) => (-5.0, -1.5),
                (Pangolin, _) => (0.0, 0.0),
                (Maneater, _) => (-1.0, 4.0),
                (Sandshark, _) => (-8.0, -5.5),
                (Hakulaq, _) => (-6.5, -4.0),
                (Dagon, _) => (2.0, -2.0),
                (Lavadrake, _) => (3.0, -5.0),
                (Icedrake, _) => (-0.5, -8.0),
                (Basilisk, _) => (0.5, -3.0),
                (Deadwood, _) => (-1.0, 4.0),
                (Mossdrake, _) => (3.0, -5.0),
                (Driggle, _) => (-2.0, -5.0),
                (Snaretongue, _) => (-7.0, -7.0),
                (Hydra, _) => (1.0, -2.0),
            },
            chest: match (body.species, body.body_type) {
                (Crocodile, _) => (0.0, 5.0),
                (SeaCrocodile, _) => (0.0, 5.0),
                (Alligator, _) => (0.0, 5.0),
                (Salamander, _) => (0.0, 5.0),
                (Elbst, _) => (0.0, 5.0),
                (Monitor, _) => (0.0, 5.0),
                (Asp, _) => (0.0, 8.0),
                (Tortoise, _) => (0.0, 11.0),
                (Rocksnapper, _) => (0.0, 18.5),
                (Rootsnapper, _) => (0.0, 18.5),
                (Reefsnapper, _) => (0.0, 18.5),
                (Pangolin, _) => (0.0, 7.0),
                (Maneater, _) => (0.0, 12.0),
                (Sandshark, _) => (0.0, 20.0),
                (Hakulaq, _) => (0.0, 13.5),
                (Dagon, _) => (0.0, 13.5),
                (Lavadrake, _) => (0.0, 16.5),
                (Icedrake, _) => (0.0, 16.5),
                (Basilisk, _) => (0.0, 15.0),
                (Deadwood, _) => (0.0, 12.0),
                (Mossdrake, _) => (0.0, 16.5),
                (Driggle, _) => (0.0, 8.0),
                (Snaretongue, _) => (-8.0, 9.0),
                (Hydra, _) => (0.0, 16.0),
            },
            tail_rear: match (body.species, body.body_type) {
                (Crocodile, _) => (-12.5, -1.0),
                (SeaCrocodile, _) => (-12.5, -1.0),
                (Alligator, _) => (-13.0, -1.0),
                (Salamander, _) => (-6.5, 0.0),
                (Elbst, _) => (-6.5, 0.0),
                (Monitor, _) => (-12.0, 0.0),
                (Asp, _) => (-14.0, -2.0),
                (Tortoise, _) => (-10.0, -1.5),
                (Rocksnapper, _) => (-14.5, -2.0),
                (Rootsnapper, _) => (-14.5, -2.0),
                (Reefsnapper, _) => (-14.5, -2.0),
                (Pangolin, _) => (-7.0, -3.0),
                (Maneater, _) => (-15.0, 4.0),
                (Sandshark, _) => (-10.0, 0.5),
                (Hakulaq, _) => (-9.0, -2.0),
                (Dagon, _) => (-9.0, -2.0),
                (Lavadrake, _) => (-12.0, -2.0),
                (Icedrake, _) => (-12.0, 1.0),
                (Basilisk, _) => (-10.0, -4.0),
                (Deadwood, _) => (-15.0, 4.0),
                (Mossdrake, _) => (-12.0, -2.0),
                (Driggle, _) => (-4.0, 0.0),
                (Snaretongue, _) => (5.0, 0.0),
                (Hydra, _) => (-16.0, -1.0),
            },
            tail_rear_length: match (body.species, body.body_type) {
                // TODO: Tweak tails as needed
                (Crocodile, _) => 1.0,
                (SeaCrocodile, _) => 1.0,
                (Alligator, _) => 1.0,
                (Salamander, _) => 1.0,
                (Elbst, _) => 1.0,
                (Monitor, _) => 1.0,
                (Asp, _) => 1.0,
                (Tortoise, _) => 1.0,
                (Rocksnapper, _) => 1.0,
                (Rootsnapper, _) => 1.0,
                (Reefsnapper, _) => 1.0,
                (Pangolin, _) => 1.0,
                (Maneater, _) => 1.0,
                (Sandshark, _) => 1.0,
                (Hakulaq, _) => 1.0,
                (Dagon, _) => 1.0,
                (Lavadrake, _) => 1.0,
                (Icedrake, _) => 1.0,
                (Basilisk, _) => 1.0,
                (Deadwood, _) => 1.0,
                (Mossdrake, _) => 1.0,
                (Driggle, _) => 1.0,
                (Snaretongue, _) => 1.0,
                (Hydra, _) => 10.0,
            },
            tail_front: match (body.species, body.body_type) {
                (Crocodile, _) => (-6.0, 0.0),
                (SeaCrocodile, _) => (-6.0, 0.0),
                (Alligator, _) => (-5.0, 0.0),
                (Salamander, _) => (-7.5, 0.0),
                (Elbst, _) => (-7.0, 0.0),
                (Monitor, _) => (-6.5, 0.0),
                (Asp, _) => (-6.0, -2.0),
                (Tortoise, _) => (-13.0, -3.5),
                (Rocksnapper, _) => (-13.5, -6.5),
                (Rootsnapper, _) => (-13.5, -6.5),
                (Reefsnapper, _) => (-13.5, -6.5),
                (Pangolin, _) => (-7.5, -0.5),
                (Maneater, _) => (-1.0, 4.0),
                (Sandshark, _) => (-13.0, -8.0),
                (Hakulaq, _) => (-6.0, -5.5),
                (Dagon, _) => (-9.0, -2.0),
                (Lavadrake, _) => (-7.0, -4.5),
                (Icedrake, _) => (-7.0, -4.5),
                (Basilisk, _) => (-6.5, -5.5),
                (Deadwood, _) => (-1.0, 4.0),
                (Mossdrake, _) => (-7.0, -4.5),
                (Driggle, _) => (-5.5, -4.0),
                (Snaretongue, _) => (5.0, -2.0),
                (Hydra, _) => (-14.0, -7.5),
            },
            feet_f: match (body.species, body.body_type) {
                (Crocodile, _) => (3.5, 6.0, -1.0),
                (SeaCrocodile, _) => (3.5, 6.0, -1.0),
                (Alligator, _) => (4.5, 4.25, -1.0),
                (Salamander, _) => (5.0, 4.5, -2.0),
                (Elbst, _) => (5.0, 4.5, -2.0),
                (Monitor, _) => (3.0, 5.0, 0.0),
                (Asp, _) => (1.5, 4.0, -1.0),
                (Tortoise, _) => (5.5, 6.5, -3.0),
                (Rocksnapper, _) => (7.5, 5.0, -8.5),
                (Rootsnapper, _) => (7.5, 5.0, -8.5),
                (Reefsnapper, _) => (7.5, 5.0, -8.5),
                (Pangolin, _) => (5.5, 5.5, -1.0),
                (Maneater, _) => (4.5, 4.0, -5.5),
                (Sandshark, _) => (5.5, 2.0, -8.0),
                (Hakulaq, _) => (4.5, 2.0, -4.5),
                (Dagon, _) => (4.5, 2.0, -4.5),
                (Lavadrake, _) => (4.5, 4.0, -6.5),
                (Icedrake, _) => (4.5, 4.0, -6.5),
                (Basilisk, _) => (6.5, 4.0, -2.0),
                (Deadwood, _) => (3.5, 4.0, -5.0),
                (Mossdrake, _) => (4.5, 4.0, -6.5),
                (Driggle, _) => (4.5, 2.5, -4.0),
                (Snaretongue, _) => (6.5, 6.5, 1.0),
                (Hydra, _) => (13.0, 7.0, -3.0),
            },
            feet_b: match (body.species, body.body_type) {
                (Crocodile, _) => (3.5, -6.0, -1.0),
                (SeaCrocodile, _) => (3.5, -6.0, -1.0),
                (Alligator, _) => (4.5, -5.5, -1.0),
                (Salamander, _) => (3.0, -6.0, -2.0),
                (Elbst, _) => (3.0, -6.0, -2.0),
                (Monitor, _) => (2.5, -6.5, 0.0),
                (Asp, _) => (2.5, -5.5, -1.0),
                (Tortoise, _) => (5.5, -11.5, -3.0),
                (Rocksnapper, _) => (8.0, -12.0, -9.5),
                (Rootsnapper, _) => (8.0, -12.0, -9.5),
                (Reefsnapper, _) => (8.0, -12.0, -9.5),
                (Pangolin, _) => (6.5, -3.5, -1.0),
                (Maneater, _) => (4.5, -2.5, -3.0),
                (Sandshark, _) => (3.5, -15.0, -14.0),
                (Hakulaq, _) => (3.5, -8.0, -4.5),
                (Dagon, _) => (3.5, -8.0, -4.5),
                (Lavadrake, _) => (3.5, -8.0, -6.5),
                (Icedrake, _) => (3.5, -8.0, -6.5),
                (Basilisk, _) => (5.5, -6.5, -2.0),
                (Deadwood, _) => (3.5, -6.0, -5.0),
                (Mossdrake, _) => (3.5, -8.0, -6.5),
                (Driggle, _) => (3.5, -3.5, -5.0),
                (Snaretongue, _) => (1.5, 1.5, 2.0),
                (Hydra, _) => (5.0, -6.5, -5.0),
            },
            lean: match (body.species, body.body_type) {
                (Pangolin, _) => (0.4, 0.0),
                _ => (0.0, 1.0),
            },
            scaler: match (body.species, body.body_type) {
                (Crocodile, _) => 1.05,
                (SeaCrocodile, _) => 1.05,
                (Alligator, _) => 1.12,
                (Salamander, _) => 1.12,
                (Elbst, _) => 1.12,
                (Monitor, _) => 0.9,
                (Asp, _) => 1.12,
                (Rocksnapper, _) => 1.12,
                (Rootsnapper, _) => 1.12,
                (Reefsnapper, _) => 1.12,
                (Hakulaq, _) => 1.05,
                (Dagon, _) => 1.05,
                (Pangolin, _) => 1.05,
                (Maneater, _) => 1.12,
                (Lavadrake, _) => 1.12,
                (Icedrake, _) => 1.12,
                (Basilisk, _) => 1.3,
                (Mossdrake, _) => 1.12,
                (Snaretongue, _) => 1.0,
                (Hydra, _) => 1.5,
                _ => 0.9,
            },
            tempo: match (body.species, body.body_type) {
                (Crocodile, _) => 0.7,
                (SeaCrocodile, _) => 0.7,
                (Alligator, _) => 0.7,
                (Salamander, _) => 0.85,
                (Elbst, _) => 0.85,
                (Monitor, _) => 1.4,
                (Tortoise, _) => 0.7,
                (Rocksnapper, _) => 0.7,
                (Rootsnapper, _) => 0.7,
                (Reefsnapper, _) => 0.7,
                (Hakulaq, _) => 1.2,
                (Dagon, _) => 1.2,
                (Pangolin, _) => 1.15,
                (Maneater, _) => 0.9,
                (Lavadrake, _) => 1.1,
                (Icedrake, _) => 1.1,
                (Basilisk, _) => 0.8,
                (Mossdrake, _) => 1.1,
                (Snaretongue, _) => 0.7,
                (Hydra, _) => 0.6,
                _ => 1.0,
            },
            // bool to special case Snaretongue
            tongue_for_tail: matches!(body.species, Snaretongue),
        }
    }
}
fn mount_point(body: &Body) -> Vec3<f32> {
    use comp::quadruped_low::Species::*;
    match (body.species, body.body_type) {
        (Crocodile, _) => (0.0, 3.5, 3.5),
        (Alligator, _) => (0.0, 2.5, 3.0),
        (Salamander, _) => (0.0, 2.0, 4.0),
        (Monitor, _) => (0.0, 0.0, 3.0),
        (Asp, _) => (0.0, 2.0, 4.0),
        (Tortoise, _) => (0.0, 0.0, 6.0),
        (Pangolin, _) => (0.0, -1.0, 4.5),
        (Maneater, _) => (0.0, 4.0, -7.5),
        (Sandshark, _) => (0.0, -6.0, 0.0),
        (Hakulaq, _) => (0.0, 5.0, 2.5),
        (Lavadrake, _) => (0.0, 2.0, -0.5),
        (Basilisk, _) => (0.0, -1.5, 8.0),
        (Deadwood, _) => (0.0, -2.0, 3.0),
        (Icedrake, _) => (0.0, -7.0, 6.0),
        (SeaCrocodile, _) => (1.0, 3.5, 6.0),
        (Dagon, _) => (1.0, 9.5, 0.5),
        (Rocksnapper, _) => (0.0, 7.0, 8.5),
        (Rootsnapper, _) => (0.0, -2.0, 12.5),
        (Reefsnapper, _) => (0.0, 2.0, 4.5),
        (Elbst, _) => (0.0, 2.0, 3.0),
        (Mossdrake, _) => (0.0, 2.0, 3.5),
        (Driggle, _) => (0.0, 6.0, 2.0),
        (Snaretongue, _) => (0.0, 2.0, 6.0),
        (Hydra, _) => (0.0, 2.0, 4.0),
    }
    .into()
}

pub fn quadruped_low_alpha(
    next: &mut QuadrupedLowSkeleton,
    _s_a: &SkeletonAttr,
    stage_section: StageSection,
    anim_time: f32,
    global_time: f32,
    timer: f32,
) {
    let (movement1base, movement2base, movement3) = match stage_section {
        StageSection::Buildup => (anim_time.sqrt(), 0.0, 0.0),
        StageSection::Action => (1.0, anim_time.powi(4), 0.0),
        StageSection::Recover => (1.0, 1.0, anim_time),
        _ => (0.0, 0.0, 0.0),
    };
    let pullback = 1.0 - movement3;
    let subtract = global_time - timer;
    let check = subtract - subtract.trunc();
    let mirror = (check - 0.5).signum();
    let twitch3 = (mirror * movement3 * 9.0).sin();
    let movement1 = mirror * movement1base * pullback;
    let movement2 = mirror * movement2base * pullback;
    let movement1abs = movement1base * pullback;
    let movement2abs = movement2base * pullback;

    // Center head
    next.head_c_upper.orientation = Quaternion::rotation_z(twitch3 * -0.7);

    next.head_c_lower.orientation =
        Quaternion::rotation_x(movement1abs * 0.35 + movement2abs * -0.9)
            * Quaternion::rotation_y(movement1 * 0.7 + movement2 * -1.0);

    next.jaw_c.orientation = Quaternion::rotation_x(movement1abs * -0.5 + movement2abs * 0.5);

    // Left head
    next.head_l_upper.orientation = Quaternion::rotation_z(twitch3 * -0.7);

    next.head_l_lower.orientation =
        Quaternion::rotation_x(movement1abs * 0.35 + movement2abs * -0.9)
            * Quaternion::rotation_y(movement1 * 0.7 + movement2 * -1.0);

    next.jaw_l.orientation = Quaternion::rotation_x(movement1abs * -0.5 + movement2abs * 0.5);

    // Right head
    next.head_r_upper.orientation = Quaternion::rotation_z(twitch3 * -0.7);

    next.head_r_lower.orientation =
        Quaternion::rotation_x(movement1abs * 0.35 + movement2abs * -0.9)
            * Quaternion::rotation_y(movement1 * 0.7 + movement2 * -1.0);

    next.jaw_r.orientation = Quaternion::rotation_x(movement1abs * -0.5 + movement2abs * 0.5);

    next.chest.orientation = Quaternion::rotation_y(movement1 * -0.08 + movement2 * 0.15)
        * Quaternion::rotation_z(movement1 * -0.2 + movement2 * 0.6);

    next.tail_front.orientation =
        Quaternion::rotation_x(0.15) * Quaternion::rotation_z(movement1 * -0.4 + movement2 * -0.2);

    next.tail_rear.orientation =
        Quaternion::rotation_x(-0.12) * Quaternion::rotation_z(movement1 * -0.4 + movement2 * -0.2);
}

pub fn quadruped_low_beta(
    next: &mut QuadrupedLowSkeleton,
    _s_a: &SkeletonAttr,
    stage_section: StageSection,
    anim_time: f32,
    global_time: f32,
    timer: f32,
) {
    let (movement1base, movement2base, movement3) = match stage_section {
        StageSection::Buildup => (anim_time.sqrt(), 0.0, 0.0),
        StageSection::Action => (1.0, anim_time.powi(4), 0.0),
        StageSection::Recover => (1.0, 1.0, anim_time),
        _ => (0.0, 0.0, 0.0),
    };
    let pullback = 1.0 - movement3;
    let subtract = global_time - timer;
    let check = subtract - subtract.trunc();
    let mirror = (check - 0.5).signum();
    let twitch3 = (mirror * movement3 * 9.0).sin();
    let movement1 = mirror * movement1base * pullback;
    let movement2 = mirror * movement2base * pullback;
    let movement1abs = movement1base * pullback;
    let movement2abs = movement2base * pullback;

    // Center head
    next.head_c_upper.orientation = Quaternion::rotation_z(twitch3 * 0.2);

    next.head_c_lower.orientation =
        Quaternion::rotation_x(movement1abs * 0.15 + movement2abs * -0.6)
            * Quaternion::rotation_y(movement1 * -0.1 + movement2 * 0.15);

    next.jaw_c.orientation = Quaternion::rotation_x(movement1abs * -0.9 + movement2abs * 0.9);

    // Left head
    next.head_l_upper.orientation = Quaternion::rotation_z(twitch3 * 0.2);

    next.head_l_lower.orientation =
        Quaternion::rotation_x(movement1abs * 0.15 + movement2abs * -0.6)
            * Quaternion::rotation_y(movement1 * -0.1 + movement2 * 0.15);

    next.jaw_l.orientation = Quaternion::rotation_x(movement1abs * -0.9 + movement2abs * 0.9);

    // Right head
    next.head_r_upper.orientation = Quaternion::rotation_z(twitch3 * 0.2);

    next.head_r_lower.orientation =
        Quaternion::rotation_x(movement1abs * 0.15 + movement2abs * -0.6)
            * Quaternion::rotation_y(movement1 * -0.1 + movement2 * 0.15);

    next.jaw_r.orientation = Quaternion::rotation_x(movement1abs * -0.9 + movement2abs * 0.9);

    next.chest.orientation = Quaternion::rotation_y(movement1 * 0.08 + movement2 * -0.15)
        * Quaternion::rotation_z(movement1 * 0.2 + movement2 * -0.3);

    next.tail_front.orientation =
        Quaternion::rotation_x(0.15) * Quaternion::rotation_z(movement1 * 0.4 + movement2 * 0.2);

    next.tail_rear.orientation =
        Quaternion::rotation_x(-0.12) * Quaternion::rotation_z(movement1 * 0.4 + movement2 * 0.2);
}
