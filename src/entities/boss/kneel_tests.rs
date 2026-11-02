use super::*;

fn update(b: &mut Boss, zone: &crate::world::Zone, fx: &mut Fx) {
    fx.sounds.clear();
    let (mut hits, mut projectiles) = (Vec::new(), Vec::new());
    b.update(&mut EnemyCtx {
        player: (b.body.x + 90.0, b.body.y + 10.0),
        player_alive: true,
        zone,
        fx,
        hits: &mut hits,
        projectiles: &mut projectiles,
        t: 0,
    });
    assert!(
        hits.is_empty(),
        "the vulnerable kneel must cancel hostile attacks"
    );
}

#[test]
fn all_real_stagger_entries_contact_before_opening_and_preserve_grounded_hands() {
    let zone = crate::world::Zone::new(crate::world::ZoneId::Summit, 60, 40);
    for phase in 0..3 {
        let mut b = Boss::new(192.0, 200.0);
        b.mode = Mode::Fight;
        b.phase = phase;
        b.body_dy = 0.0;
        b.cool = 10_000;
        b.pylons_up = true;
        if phase == 2 {
            b.attack = Some(Attack::Beam { t: 3, twin: true });
        }
        for (i, f) in b.fists.iter_mut().enumerate() {
            f.x = 125.0 + i as f32 * 149.0;
            f.y = 232.0 + i as f32 * 5.0;
            f.z = 0.0;
            f.st = St::Stuck(4, 70);
            f.crystal_hp = 0;
        }
        let contacts = b.fists.each_ref().map(|f| (f.x, f.y));
        let mut fx = Fx::new(7);
        update(&mut b, &zone, &mut fx);
        assert_eq!(b.mode, Mode::Stagger(0));
        let end = 38 + TEMPO[phase].window;
        let mut vulnerable = Vec::new();
        let mut impacts = Vec::new();
        for t in 0..end + 34 {
            update(&mut b, &zone, &mut fx);
            for (f, p) in b.fists.iter().zip(contacts) {
                assert_eq!(
                    (f.x, f.y, f.z),
                    (p.0, p.1, 0.0),
                    "ground contact phase{phase} tick{t}"
                );
                assert!(
                    matches!(f.st, St::Limp(_)),
                    "hands release only after complete extension"
                );
            }
            if fx.sounds.iter().any(|s| s.name == "boss_slam") {
                impacts.push(t);
                assert_eq!(b.body_dy.round() as i32, pose::rig().fold_max);
                assert_eq!(b.heart.out, 0.0);
                let body = b.body_sprite();
                assert_eq!(art::bottom(body) + b.body_dy.round() as i32, 0);
                assert_eq!(b.support_contacts().len(), 2);
            }
            if b.hurtboxes().iter().any(|v| v.3 == PART_HEART) {
                vulnerable.push(t + 1);
            }
            assert!(b.beams.is_empty() && b.shards.is_empty() && b.rings.is_empty());
        }
        assert_eq!(impacts, vec![kneel::CONTACT]);
        assert_eq!(
            vulnerable,
            (30..=end + 5).collect::<Vec<_>>(),
            "unchanged playable window"
        );
        assert_eq!(b.body_dy, 0.0);
        update(&mut b, &zone, &mut fx);
        assert_eq!(b.mode, Mode::Fight);
        assert!(b.fists.iter().all(|f| matches!(f.st, St::Return(_))));
    }
}

#[test]
fn phase_clear_and_missed_window_extend_the_same_joints_before_hand_release() {
    let zone = crate::world::Zone::new(crate::world::ZoneId::Summit, 60, 40);
    let mut missed = Boss::new(192.0, 200.0);
    let mut cleared = Boss::new(192.0, 200.0);
    let end = 38 + TEMPO[0].window;
    missed.mode = Mode::Stagger(end + 14);
    cleared.mode = Mode::Transition(7);
    cleared.recover_from = kneel_depth();
    for b in [&mut missed, &mut cleared] {
        b.body_dy = kneel_depth();
        for f in &mut b.fists {
            f.z = 0.0;
            f.limp();
        }
    }
    let (mut fx_a, mut fx_b) = (Fx::new(7), Fx::new(7));
    let mut prior = kneel_depth();
    for t in 0..kneel::RISE_TICKS {
        update(&mut missed, &zone, &mut fx_a);
        update(&mut cleared, &zone, &mut fx_b);
        assert_eq!(
            missed.body_dy, cleared.body_dy,
            "shared pose at extension{t}"
        );
        assert!(missed.body_dy <= prior);
        prior = missed.body_dy;
        if prior > 0.0 {
            assert!(
                missed
                    .fists
                    .iter()
                    .chain(&cleared.fists)
                    .all(|f| matches!(f.st, St::Limp(_)))
            );
        }
    }
    assert_eq!(prior, 0.0);
}

#[test]
fn airborne_hands_lose_lift_without_sideways_skating_or_false_ground_impacts() {
    let zone = crate::world::Zone::new(crate::world::ZoneId::Summit, 60, 40);
    let mut b = Boss::new(192.0, 200.0);
    b.mode = Mode::Fight;
    b.phase = 1;
    b.pylons_up = true;
    for f in &mut b.fists {
        f.x += 24.0;
        f.y += 18.0;
        f.z = 80.0;
        f.st = St::Rest;
    }
    let contacts = b.fists.each_ref().map(|f| (f.x, f.y));
    let mut fx = Fx::new(7);
    update(&mut b, &zone, &mut fx);
    let mut prior = b.fists[0].z;
    let mut hand_impacts = 0;
    for t in 0..24 {
        update(&mut b, &zone, &mut fx);
        for (f, p) in b.fists.iter().zip(contacts) {
            assert_eq!((f.x, f.y), p);
        }
        assert!(b.fists[0].z <= prior);
        prior = b.fists[0].z;
        for sound in &fx.sounds {
            if sound.name == "boss_slam" && sound.volume == 0.45 {
                hand_impacts += 1;
                assert_eq!(b.fists[0].z, 0.0);
                assert!(t < kneel::CONTACT);
            }
        }
    }
    assert_eq!(hand_impacts, 2);
    assert_eq!(prior, 0.0);
}
