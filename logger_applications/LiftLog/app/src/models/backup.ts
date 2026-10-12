import { ProgramBlueprint } from '@/models/blueprint-models';
import { Session } from '@/models/session-models';
import { ExerciseDescriptor } from '@/models/exercise-models';

export interface BackupData {
  workouts: Session[];
  programs: Record<string, ProgramBlueprint>;
  exercises?: Record<string, ExerciseDescriptor>;
}
