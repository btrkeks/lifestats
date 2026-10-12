import PageMenu from '@/components/presentation/foundation/page-menu';
import { useAppSelector } from '@/store';
import { useTranslate } from '@tolgee/react';
import { useRouter } from 'expo-router';
import Edit from '@expo/material-symbols/edit.xml';
import Assignment from '@expo/material-symbols/assignment.xml';
import FitnessCenter from '@expo/material-symbols/fitness_center.xml';

export function usePlanNavigation() {
  const activeProgramId = useAppSelector((s) => s.program.activePlanId);
  const { push } = useRouter();

  return {
    choosePlan: () => push('/settings/program-list', { withAnchor: true }),
    editWorkouts: () => push(`/settings/manage-workouts/${activeProgramId}`, { withAnchor: true }),
  };
}

/** `onStartEmptyWorkout` starts a workout outside the plan, for the odd one-off session. */
export default function PlanMenu({ onStartEmptyWorkout }: { onStartEmptyWorkout: () => void }) {
  const { t } = useTranslate();
  const { choosePlan, editWorkouts } = usePlanNavigation();

  return (
    <PageMenu
      testID="plan-menu"
      items={[
        {
          label: t('workout.empty_workout.button'),
          icon: FitnessCenter,
          systemImage: 'dumbbell',
          onPress: onStartEmptyWorkout,
        },
        {
          label: t('plan.choose.button'),
          icon: Assignment,
          systemImage: 'list.clipboard',
          onPress: choosePlan,
        },
        {
          label: t('workout.edit_workouts.button'),
          icon: Edit,
          systemImage: 'pencil',
          onPress: editWorkouts,
        },
      ]}
    />
  );
}
