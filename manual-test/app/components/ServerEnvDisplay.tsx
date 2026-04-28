export default function ServerEnvDisplay() {
  const serverEnv = process.env.TEST_ENV ?? "(undefined)";
  const publicEnv = process.env.NEXT_PUBLIC_TEST_ENV ?? "(undefined)";

  return (
    <div className="mt-8 w-full rounded-lg border border-zinc-200 dark:border-zinc-800 overflow-hidden font-mono text-sm">
      <div className="bg-zinc-100 dark:bg-zinc-900 px-4 py-2 text-xs text-zinc-500 uppercase tracking-wide">
        .env values
      </div>
      <dl className="divide-y divide-zinc-200 dark:divide-zinc-800">
        <div className="flex items-center gap-4 px-4 py-3">
          <dt className="w-64 text-zinc-500 shrink-0">NEXT_PUBLIC_TEST_ENV</dt>
          <dd className="text-green-600 dark:text-green-400 break-all">{publicEnv}</dd>
        </div>
        <div className="flex items-center gap-4 px-4 py-3">
          <dt className="w-64 text-zinc-500 shrink-0">TEST_ENV</dt>
          <dd className="text-green-600 dark:text-green-400 break-all">{serverEnv}</dd>
        </div>
      </dl>
    </div>
  );
}
