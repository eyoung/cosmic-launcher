use std::collections::HashMap;
use std::time::SystemTime;

pub trait Recommender {
    fn record_launch(&mut self, app_id: &str, timestamp: SystemTime);
    fn get_recommendations(&self, count: usize) -> Vec<String>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recommends_most_launched_app() {
        let mut recommender = FrequencyBasedRecommender::default();
        let now = SystemTime::now();
        
        recommender.record_launch("firefox", now);
        recommender.record_launch("firefox", now);
        recommender.record_launch("firefox", now);
        recommender.record_launch("terminal", now);
        
        let recommendations = recommender.get_recommendations(1);
        
        assert_eq!(recommendations, vec!["firefox"]);
    }

    #[test]
    fn returns_multiple_recommendations_sorted_by_frequency() {
        let mut recommender = FrequencyBasedRecommender::default();
        let now = SystemTime::now();
        
        recommender.record_launch("firefox", now);
        recommender.record_launch("firefox", now);
        recommender.record_launch("firefox", now);
        recommender.record_launch("terminal", now);
        recommender.record_launch("terminal", now);
        recommender.record_launch("vscode", now);
        
        let recommendations = recommender.get_recommendations(3);
        
        assert_eq!(recommendations, vec!["firefox", "terminal", "vscode"]);
    }
}

struct FrequencyBasedRecommender {
    launch_counts: HashMap<String, usize>,
}

impl Default for FrequencyBasedRecommender {
    fn default() -> Self {
        Self {
            launch_counts: HashMap::new(),
        }
    }
}

impl Recommender for FrequencyBasedRecommender {
    fn record_launch(&mut self, app_id: &str, _timestamp: SystemTime) {
        *self.launch_counts.entry(app_id.to_string()).or_insert(0) += 1;
    }

    fn get_recommendations(&self, count: usize) -> Vec<String> {
        let mut apps: Vec<_> = self.launch_counts.iter().collect();
        apps.sort_by(|a, b| b.1.cmp(a.1));
        apps.into_iter()
            .take(count)
            .map(|(app_id, _)| app_id.clone())
            .collect()
    }
}
