use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ActivitySummaryResponse {
    pub activities: Vec<Activity>,
    pub summary: Summary,
    pub goals: Goals,
}

impl ActivitySummaryResponse {
    pub fn get_steps(&self) -> u32 {
        self.summary.steps
    }
}

#[derive(Debug, Deserialize)]
pub struct Activity {
    // TODO
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    pub calories_out: i32,
    pub activity_calories: i32,
    #[serde(rename = "caloriesBMR")]
    pub calories_bmr: i32,
    pub active_score: i32,
    pub steps: u32,
    pub floors: i32,
    pub elevation: f64,
    pub sedentary_minutes: i32,
    pub lightly_active_minutes: i32,
    pub fairly_active_minutes: i32,
    pub very_active_minutes: i32,
    pub distances: Vec<Distance>,
    pub marginal_calories: i32,
    pub resting_heart_rate: i32,
    pub heart_rate_zones: Vec<HeartRateZone>,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ActivityType {
    Total,
    Tracker,
    LoggedActivities,
    VeryActive,
    ModeratelyActive,
    LightlyActive,
    SedentaryActive,
}

#[derive(Debug, Deserialize)]
pub struct Distance {
    pub activity: ActivityType,
    pub distance: f64,
}

#[derive(Debug, Deserialize, PartialEq)]
pub enum HeartRateZoneName {
    #[serde(rename = "Out of Range")]
    OutOfRange,
    #[serde(rename = "Fat Burn")]
    FatBurn,
    Cardio,
    Peak,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeartRateZone {
    pub minutes: i32,
    pub calories_out: f64,
    pub name: HeartRateZoneName,
    pub min: i32,
    pub max: i32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Goals {
    pub calories_out: i32,
    pub steps: u32,
    pub distance: f64,
    pub floors: i32,
    pub active_minutes: i32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_fitbit_response() {
        let json_str = r#"{
            "activities": [],
            "summary": {
                "caloriesOut": 1746,
                "activityCalories": 62,
                "caloriesBMR": 668,
                "activeScore": -1,
                "steps": 27,
                "floors": 0,
                "elevation": 0.0,
                "sedentaryMinutes": 552,
                "lightlyActiveMinutes": 14,
                "fairlyActiveMinutes": 0,
                "veryActiveMinutes": 0,
                "distances": [
                    {"activity": "total", "distance": 0.0197},
                    {"activity": "tracker", "distance": 0.0197},
                    {"activity": "sedentaryActive", "distance": 0.0067},
                    {"activity": "lightlyActive", "distance": 0.013},
                    {"activity": "moderatelyActive", "distance": 0.0},
                    {"activity": "veryActive", "distance": 0.0},
                    {"activity": "loggedActivities", "distance": 0.0}
                ],
                "marginalCalories": 40,
                "restingHeartRate": 60,
                "heartRateZones": [
                    {"minutes": 412, "caloriesOut": 529.8314, "name": "Out of Range", "min": 30, "max": 114},
                    {"minutes": 1, "caloriesOut": 4.9539, "name": "Fat Burn", "min": 115, "max": 141},
                    {"minutes": 0, "caloriesOut": 0.0, "name": "Cardio", "min": 142, "max": 176},
                    {"minutes": 0, "caloriesOut": 0.0, "name": "Peak", "min": 177, "max": 220}
                ]
            },
            "goals": {
                "caloriesOut": 2545,
                "steps": 8000,
                "distance": 8.05,
                "floors": 10,
                "activeMinutes": 30
            }
        }"#;

        let response: ActivitySummaryResponse =
            serde_json::from_str(json_str).expect("Failed to parse JSON");

        // Test summary fields
        assert_eq!(response.summary.calories_out, 1746);
        assert_eq!(response.summary.activity_calories, 62);
        assert_eq!(response.summary.calories_bmr, 668);
        assert_eq!(response.summary.active_score, -1);
        assert_eq!(response.summary.steps, 27);
        assert_eq!(response.summary.floors, 0);
        assert_eq!(response.summary.elevation, 0.0);
        assert_eq!(response.summary.sedentary_minutes, 552);
        assert_eq!(response.summary.lightly_active_minutes, 14);
        assert_eq!(response.summary.fairly_active_minutes, 0);
        assert_eq!(response.summary.very_active_minutes, 0);
        assert_eq!(response.summary.marginal_calories, 40);
        assert_eq!(response.summary.resting_heart_rate, 60);

        // Test distances
        assert_eq!(response.summary.distances.len(), 7);
        assert_eq!(response.summary.distances[0].activity, ActivityType::Total);
        assert_eq!(response.summary.distances[0].distance, 0.0197);
        assert_eq!(
            response.summary.distances[2].activity,
            ActivityType::SedentaryActive
        );
        assert_eq!(response.summary.distances[2].distance, 0.0067);

        // Test heart rate zones
        assert_eq!(response.summary.heart_rate_zones.len(), 4);
        assert_eq!(
            response.summary.heart_rate_zones[0].name,
            HeartRateZoneName::OutOfRange
        );
        assert_eq!(response.summary.heart_rate_zones[0].minutes, 412);
        assert_eq!(response.summary.heart_rate_zones[0].calories_out, 529.8314);
        assert_eq!(response.summary.heart_rate_zones[0].min, 30);
        assert_eq!(response.summary.heart_rate_zones[0].max, 114);

        assert_eq!(
            response.summary.heart_rate_zones[1].name,
            HeartRateZoneName::FatBurn
        );
        assert_eq!(response.summary.heart_rate_zones[1].minutes, 1);

        // Test goals
        assert_eq!(response.goals.calories_out, 2545);
        assert_eq!(response.goals.steps, 8000);
        assert_eq!(response.goals.distance, 8.05);
        assert_eq!(response.goals.floors, 10);
        assert_eq!(response.goals.active_minutes, 30);
    }
}
