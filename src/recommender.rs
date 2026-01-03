use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

pub trait TimeProvider {
    fn now(&self) -> SystemTime;
}

pub trait Storage {
    fn save(&self, data: &[u8]) -> Result<(), std::io::Error>;
    fn load(&self) -> Result<Vec<u8>, std::io::Error>;
}

pub struct SystemTimeProvider;

impl TimeProvider for SystemTimeProvider {
    fn now(&self) -> SystemTime {
        SystemTime::now()
    }
}

pub struct FileStorage {
    path: std::path::PathBuf,
}

impl FileStorage {
    pub fn new(path: std::path::PathBuf) -> Self {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        Self { path }
    }
}

impl Storage for FileStorage {
    fn save(&self, data: &[u8]) -> Result<(), std::io::Error> {
        std::fs::write(&self.path, data)
    }

    fn load(&self) -> Result<Vec<u8>, std::io::Error> {
        std::fs::read(&self.path)
    }
}

pub trait Recommender {
    fn record_launch(&mut self, app_id: &str, timestamp: SystemTime);
    fn get_recommendations(&self, count: usize) -> Vec<String>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    struct MockTimeProvider {
        current_time: SystemTime,
    }

    impl TimeProvider for MockTimeProvider {
        fn now(&self) -> SystemTime {
            self.current_time
        }
    }

    #[derive(Clone)]
    struct MockStorage {
        data: Rc<RefCell<Vec<u8>>>,
    }

    impl MockStorage {
        fn new() -> Self {
            Self {
                data: Rc::new(RefCell::new(Vec::new())),
            }
        }
    }

    impl Storage for MockStorage {
        fn save(&self, data: &[u8]) -> Result<(), std::io::Error> {
            *self.data.borrow_mut() = data.to_vec();
            Ok(())
        }

        fn load(&self) -> Result<Vec<u8>, std::io::Error> {
            let data = self.data.borrow();
            if data.is_empty() {
                Err(std::io::Error::new(std::io::ErrorKind::NotFound, "No data"))
            } else {
                Ok(data.clone())
            }
        }
    }

    #[test]
    fn recommends_most_launched_app() {
        let now = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1000);
        let time_provider = MockTimeProvider { current_time: now };
        let storage = MockStorage::new();
        let mut recommender = FrequencyBasedRecommender::new(time_provider, storage);
        
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
        let storage = MockStorage::new();
        let mut recommender = FrequencyBasedRecommender::new(time_provider, storage);
        
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
        let storage = MockStorage::new();
        let mut recommender = FrequencyBasedRecommender::new(time_provider, storage);
        
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

    #[test]
    fn persists_data_across_instances() {
        let now = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1000);
        let storage = MockStorage::new();
        
        // First instance: record launches
        {
            let time_provider = MockTimeProvider { current_time: now };
            let mut recommender = FrequencyBasedRecommender::new(time_provider, storage.clone());
            recommender.record_launch("firefox", now);
            recommender.record_launch("firefox", now);
            recommender.record_launch("terminal", now);
        } // recommender dropped here
        
        // Second instance: should load previous data
        {
            let time_provider = MockTimeProvider { current_time: now };
            let recommender = FrequencyBasedRecommender::new(time_provider, storage);
            let recommendations = recommender.get_recommendations(2);
            assert_eq!(recommendations, vec!["firefox", "terminal"]);
        }
    }
}

#[derive(Serialize, Deserialize)]
struct LaunchData {
    app_id: String,
    timestamps_secs: Vec<u64>,
}

pub struct FrequencyBasedRecommender<T: TimeProvider, S: Storage> {
    launches: HashMap<String, Vec<SystemTime>>,
    time_provider: T,
    storage: S,
}

impl<T: TimeProvider, S: Storage> FrequencyBasedRecommender<T, S> {
    pub fn new(time_provider: T, storage: S) -> Self {
        let mut recommender = Self {
            launches: HashMap::new(),
            time_provider,
            storage,
        };
        // Try to load existing data
        let _ = recommender.load_from_storage();
        recommender
    }

    fn load_from_storage(&mut self) -> Result<(), std::io::Error> {
        let data = self.storage.load()?;
        let launch_data: Vec<LaunchData> = serde_json::from_slice(&data)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        
        self.launches = launch_data
            .into_iter()
            .map(|ld| {
                let timestamps = ld
                    .timestamps_secs
                    .into_iter()
                    .map(|secs| UNIX_EPOCH + std::time::Duration::from_secs(secs))
                    .collect();
                (ld.app_id, timestamps)
            })
            .collect();
        Ok(())
    }

    fn save_to_storage(&self) -> Result<(), std::io::Error> {
        let launch_data: Vec<LaunchData> = self
            .launches
            .iter()
            .map(|(app_id, timestamps)| LaunchData {
                app_id: app_id.clone(),
                timestamps_secs: timestamps
                    .iter()
                    .filter_map(|t| t.duration_since(UNIX_EPOCH).ok().map(|d| d.as_secs()))
                    .collect(),
            })
            .collect();
        
        let data = serde_json::to_vec(&launch_data)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        self.storage.save(&data)
    }
}

impl<T: TimeProvider, S: Storage> Recommender for FrequencyBasedRecommender<T, S> {
    fn record_launch(&mut self, app_id: &str, timestamp: SystemTime) {
        self.launches
            .entry(app_id.to_string())
            .or_insert_with(Vec::new)
            .push(timestamp);
        
        // Auto-save after recording
        let _ = self.save_to_storage();
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

impl<T: TimeProvider, S: Storage> FrequencyBasedRecommender<T, S> {
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
