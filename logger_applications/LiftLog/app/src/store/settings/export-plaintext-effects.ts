import { RecordedCardioExercise, RecordedExercise, Session } from '@/models/session-models';
import { AddEffectFn } from '@/store/store';
import { exportPlainText } from '@/store/settings';
import Enumerable from 'linq';
import { match } from 'ts-pattern';
import BigNumber from 'bignumber.js';
import { jsonToCSV } from 'react-native-csv';
import { shortFormatWeightUnit } from '@/models/weight';
import { DateTimeFormatter, LocalDateTime } from '@js-joda/core';
import { readSessions } from '@/db/sessions';
import { TemporalComparer } from '@/models/comparers';
import { getSessionReferenceTime } from '@/store/stored-sessions';

export function addExportPlaintextEffects(addEffect: AddEffectFn) {
  addEffect(exportPlainText, async ({ payload: { format } }, { extra: { db, fileExportService } }) => {
    const sessions = Enumerable.from(readSessions(db).sessions).orderByDescending(
      (x) => getSessionReferenceTime(x),
      TemporalComparer,
    );
    const now = LocalDateTime.now()
      .withNano(0)
      .format(DateTimeFormatter.ISO_LOCAL_DATE_TIME)
      .replaceAll(':', '')
      .replaceAll('T', '_')
      .replaceAll('-', '');
    const [fileName, bytes, contentType] = await match(format)
      .with('CSV', async () => [`liftlog-export.${now}.csv`, await exportToCsv(sessions), 'text/csv'] as const)
      .with(
        'JSON',
        async () => [`liftlog-export.${now}.json`, await exportToJson(sessions), 'application/json'] as const,
      )
      .exhaustive();

    await fileExportService.exportBytes(fileName, bytes, contentType);
  });
}

async function exportToJson(sessions: Enumerable.IEnumerable<Session>): Promise<Uint8Array> {
  const exportedSessions = sessions
    .select((session) => ({
      id: session.id,
      blueprint: { name: session.blueprint.name, notes: session.blueprint.notes },
      date: session.date,
      bodyweight: session.bodyweight,
      recordedExercises: session.recordedExercises.map(exportedExercise),
    }))
    .toArray();
  return new TextEncoder().encode(JSON.stringify(exportedSessions));
}

/**
 * What was planned and done, without the rep targets, progression rules and failure rest that stored
 * weighted exercises still carry for compatibility.
 */
function exportedExercise(exercise: RecordedExercise) {
  if (exercise instanceof RecordedCardioExercise) {
    return exercise.toJSON();
  }
  const { blueprint, potentialSets, ...performed } = exercise.toJSON();
  return {
    ...performed,
    blueprint: {
      type: blueprint.type,
      name: blueprint.name,
      sets: blueprint.plannedSets.length,
      resistance: blueprint.resistance,
      restBetweenSets: { minRest: blueprint.restBetweenSets.minRest, maxRest: blueprint.restBetweenSets.maxRest },
      supersetWithNext: blueprint.supersetWithNext,
      notes: blueprint.notes,
      link: blueprint.link,
    },
    potentialSets: potentialSets.map((set) => ({ weight: set.weight, set: set.set })),
  };
}

async function exportToCsv(sessions: Enumerable.IEnumerable<Session>): Promise<Uint8Array> {
  const exportedSets = sessions
    .selectMany((session) => session.recordedExercises.map((exercise) => ({ session, exercise })))
    .selectMany(({ session, exercise }) => ExportedSetCsvRow.fromModel(session, exercise));
  const csvString = jsonToCSV(exportedSets.toArray());
  return new TextEncoder().encode(csvString);
}

class ExportedSetCsvRow {
  constructor(
    public SessionId: string,
    public Timestamp: string,
    public Exercise: string,
    public Weight: BigNumber | '',
    public WeightUnit: string,
    public Reps: number,
    public Notes: string,
  ) {}
  static fromModel(session: Session, exercise: RecordedExercise): ExportedSetCsvRow[] {
    // TODO: What do we do about cardio?
    if (exercise instanceof RecordedCardioExercise) {
      return [];
    }
    return exercise.potentialSets
      .filter((set) => set.set)
      .map(
        (set) =>
          new ExportedSetCsvRow(
            session.id,
            set.set!.completionDateTime.toString(),
            exercise.blueprint.name,
            // An exercise with no load has no weight to report.
            exercise.tracksResistance ? set.weight.value : '',
            exercise.tracksResistance ? shortFormatWeightUnit(set.weight.unit) : '',
            set.set!.repsCompleted,
            exercise.notes ?? '',
          ),
      );
  }
}
