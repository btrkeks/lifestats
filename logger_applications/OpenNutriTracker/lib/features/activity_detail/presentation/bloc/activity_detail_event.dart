part of 'activity_detail_bloc.dart';

abstract class ActivityDetailEvent extends Equatable {
  const ActivityDetailEvent();
}

class LoadActivityDetailEvent extends ActivityDetailEvent {
  final PhysicalActivityEntity physicalActivity;

  const LoadActivityDetailEvent(this.physicalActivity);

  @override
  List<Object?> get props => [];
}
