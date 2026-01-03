use std::collections::HashMap;
use std::time::SystemTime;

pub trait TimeProvider {
    fn now(&self) -> SystemTime;
}

pub struct SystemTimeProvider;

impl TimeProvider for SystemTimeProvider {
    fn now(&self) -> SystemTime {
        SystemTime::now()
    }
}

pub trait Recommender {
    fn record_launch(&mut self, app_id: &str, timestamp: SystemTime);
    fn get_recommendations(&self, count: usize) -> Vec<String>;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockTimeProvider {
        current_time: SystemTime,
    }

    impl TimeProvider for MockTimeProvider {
        fn now(&self) -> SystemTime {
            self.current_time
        }
    }

    #[test]
    fn recommends_most_launched_app() {
        let now = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1000);
        let time_provider = MockTimeProvider { current_time: now };
        let mut recommender = FrequencyBasedRecommender::new(time_provider);
        
        recommender.record_launch("firefox", now);
        recommender.record_launch("firefox", now);
        recommender.record_launch("firefox", now);
        recommender.record_launch("terminal", now);
        
        let recommendations = recommender.get_recommendations(1);
        
        assert_eq!(recommendations, vec!["firefox"]);
    }

    #[test]
    fn returns_multiple_recommendations_sorted_by_frequency() {
        let now = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1000);
        let time_provider = MockTimeProvider { current_time: now };
        let mut recommender = FrequencyBasedRecommender::new(time_provider);
        
        recommender.record_launch("firefox", now);
        recommender.record_launch("firefox", now);
        recommender.record_launch("firefox", now);
        recommender.record_launch("terminal", now);
        recommender.record_launch("terminal", now);
        recommender.record_launch("vscode", now);
        
        let recommendations = recommender.get_recommendations(3);
        
        assert_eq!(recommendations, vec!["firefox", "terminal", "vscode"]);
    }

    #[test]
    fn weighs_recent_launches_higher_than_old_frequent_launches() {
        let now = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_000_000);
        let time_provider = MockTimeProvider { current_time: now };
        let mut recommender = FrequencyBasedRecommender::new(time_provider);
        
        // Firefox launched 5 times 30 days ago
        let old_time = now - std::time::Duration::from_secs(30 * 86400);
        // Terminal launched only 2 times 1 hour ago
        let recent_time = now - std::time::Duration::from_secs(3600);
        
        recommender.record_launch("firefox", old_time);
        recommender.record_launch("firefox", old_time);
        recommender.record_launch("firefox", old_time);
        recommender.record_launch("firefox", old_time);
        recommender.record_launch("firefox", old_time);
        
        recommender.record_launch("terminal", recent_time);
        recommender.record_launch("terminal", recent_time);
        
        let recommendations = recommender.get_recommendations(2);
        
        assert_eq!(recommendations, vec!["terminal", "firefox"]);
    }
}

struct FrequencyBasedRecommender<T: TimeProvider> {
    launches: HashMap<String, Vec<SystemTime>>,
    time_provider: T,
}

impl<T: TimeProvider> FrequencyBasedRecommender<T> {
    fn new(time_provider: T) -> Self {
        Self {
            launches: HashMap::new(),
            time_provider,
        }
    }
}

impl<T: TimeProvider> Recommender for FrequencyBasedRecommender<T> {
    fn record_launch(&mut self, app_id: &str, timestamp: SystemTime) {
        self.launches
            .entry(app_id.to_string())
            .or_insert_with(Vec::new)
            .push(timestamp);
    }

    fn get_recommendations(&self, count: usize) -> Vec<String> {
        let now = self.time_provider.now();
        let mut app_scores: Vec<_> = self.launches
            .iter()
            .map(|(app_id, timestamps)| {
                let score = self.calculate_score(timestamps, now);
                (app_id.clone(), score)
            })
            .collect();
        
        app_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        app_scores.into_iter()
            .take(count)
            .map(|(app_id, _)| app_id)
            .collect()
    }
}

impl<T: TimeProvider> FrequencyBasedRecommender<T> {
    fn calculate_score(&self, timestamps: &[SystemTime], now: SystemTime) -> f64 {
        // Exponential decay: score = sum of exp(-age_in_days / half_life)
        // Using a half-life of 7 days
        const HALF_LIFE_DAYS: f64 = 7.0;
        const SECONDS_PER_DAY: f64 = 86400.0;
        
        timestamps.iter().map(|&timestamp| {
            let age_seconds = now.duration_since(timestamp)
                .unwrap_or(std::time::Duration::ZERO)
                .as_secs_f64();
            let age_days = age_seconds / SECONDS_PER_DAY;
            (-age_days / HALF_LIFE_DAYS).exp()
        }).sum()
    }
}
