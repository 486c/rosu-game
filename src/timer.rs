use std::time::{Instant, Duration};

pub struct Timer {
    now: Instant,

    /// Milliseconds 
    pub last_time: f64,

    paused: bool,
}

impl Timer {
    pub fn new() -> Self {
        Self {
            now: Instant::now(),
            last_time: 0.0,
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

    pub fn get_time(&self) -> f64 {
        self.last_time
    }

    pub fn set_time(&mut self, time: f64) {
        self.last_time = time;
    }

    pub fn reset_time(&mut self) {
        self.last_time = 0.0;
        self.paused = true;
    }
    
    /// Updates and returns current time
    pub fn update(&mut self) -> f64 {
        // TODO refactor
        if self.paused {
            return self.last_time
        };

        let now = Instant::now();

        let diff = now.duration_since(self.now);

        // Converting to millis
        self.last_time += diff.as_secs_f64() * 1000.0;

        self.now = now;

        self.last_time
    }
}

#[test]
fn test_timer_logic() {
    let mut clock = Timer::new();

    std::thread::sleep(Duration::from_millis(15));

    assert!(clock.update() == 0.0);

    clock.unpause();

    std::thread::sleep(Duration::from_millis(15));

    let expected = clock.update();

    assert!(expected > 13.0 && expected < 17.0);

    clock.pause();

    assert!(clock.update() == expected)
}
