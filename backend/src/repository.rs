// pub trait Repository {
//     fn get_statistic_by_id(&self, id: u32) -> Result<Statistic, AppError>;
//     fn log_value(&self, statistic: &Statistic, date: NaiveDate, value: impl Into<Value>) -> Result<(), AppError>;
//     // ... other methods
// }
//
// // Implement for SQLite
// pub struct SqliteStatisticRepository {
//     conn: Connection,
// }
//
// impl StatisticRepository for SqliteStatisticRepository {
//     // Implement methods
// }
//
// // Use the repository in other parts of the code
// let repo = SqliteStatisticRepository::new(conn);
// let statistic = repo.get_statistic_by_id(args.id)?;
