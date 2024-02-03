use std::time::{Instant, Duration};

pub struct Timer {
    now: Instant,
    last_time: u128, // Milliseconds
    paused: bool,
}

impl Timer {
    pub fn new() -> Self {
        Self {
            now: Instant::now(),
            last_time: 0,
            paused: true,
        }
    }

    pub fn is_paused(&self) -> bool {
        self.paused == true
    }

    pub fn pause(&mut self) {
        self.paused = true;
    }

    pub fn unpause(&mut self) {
        self.paused = false;

        self.now = Instant::now();
    }

    pub fn get_time(&self) -> u128 {
        self.last_time
    }
    
    /// Updates and returns current time
    pub fn update(&mut self) -> u128 {
        if self.paused {
            return self.last_time
        };

        let now = Instant::now();

        let diff = now.duration_since(self.now);

        self.last_time += diff.as_millis();

        self.now = now;

        self.last_time
    }
}

#[test]
fn test_timer_logic() {
    let mut clock = Timer::new();

    std::thread::sleep(Duration::from_millis(15));

    assert!(clock.update() == 0);

    clock.unpause();

    std::thread::sleep(Duration::from_millis(15));

    let expected = clock.update();

    assert!(expected > 13 && expected < 17);

    clock.pause();

    assert!(clock.update() == expected)
}
