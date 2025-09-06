use lambda_control::autoscaler::plan_scale;

#[test]
fn no_work_no_scale() {
    assert_eq!(plan_scale(0, 0, 0), (0, 0));
    assert_eq!(plan_scale(0, 5, 3), (0, 0));
}

#[test]
fn idle_covers_demand() {
    assert_eq!(plan_scale(3, 3, 2), (0, 0));
    assert_eq!(plan_scale(2, 5, 0), (0, 0));
}

#[test]
fn restart_stopped_first() {
    // Need 4, idle 1 => need 3; stopped 2 => restart 2, create 1
    assert_eq!(plan_scale(4, 1, 2), (2, 1));
    // Need 5, idle 0, stopped 10 => restart 5, create 0
    assert_eq!(plan_scale(5, 0, 10), (5, 0));
}

#[test]
fn create_when_no_stopped() {
    assert_eq!(plan_scale(3, 1, 0), (0, 2));
}
