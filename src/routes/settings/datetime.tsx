import { useEffect, useMemo, useState } from "react";
import { createFileRoute, useNavigate } from "@tanstack/react-router";
import { FocusContext, useBackHandler, usePageFocus } from "@/lib/focus";
import { useActionHints } from "@/lib/hints";
import { useDateTimeStatus, useSetNtpEnabled, useSetTime, useSetTimezone, useTimezones } from "@/hooks/use-datetime";
import { List, ListRow } from "@/components/list";
import { TextFieldRow } from "@/components/text-field-row";
import { Modal } from "@/components/modal";
import { SearchField } from "@/components/search-field";
import { Header } from "@/components/header";

export const Route = createFileRoute("/settings/datetime")({
  component: RouteComponent,
});

// Ticks once a second, formatted in whichever timezone `status.timezone` currently says --
// Intl.DateTimeFormat's timeZone option renders a Date in an arbitrary IANA zone regardless of the
// process's own TZ, so this reflects a just-changed timezone selection immediately rather than
// waiting on anything OS/process-level to catch up.
function useFormattedNow(timezone: string | undefined): string {
  const [text, setText] = useState("");

  useEffect(() => {
    if (!timezone) return;
    const formatter = new Intl.DateTimeFormat(undefined, { timeZone: timezone, dateStyle: "medium", timeStyle: "medium" });
    const tick = () => setText(formatter.format(new Date()));
    tick();
    const interval = setInterval(tick, 1000);
    return () => clearInterval(interval);
  }, [timezone]);

  return text;
}

function RouteComponent() {
  const navigate = useNavigate();
  const { ref, focusKey } = usePageFocus("SETTINGS_DATETIME_SCREEN");
  useBackHandler(() => navigate({ to: "/settings" }));
  useActionHints([
    { action: "confirm", label: "Select" },
    { action: "back", label: "Back" },
  ]);

  const { data: status, error } = useDateTimeStatus();
  const now = useFormattedNow(status?.timezone);

  const setNtpEnabled = useSetNtpEnabled();
  const setTimezone = useSetTimezone();
  const setTime = useSetTime();

  const [timezoneModalOpen, setTimezoneModalOpen] = useState(false);

  return (
    <FocusContext.Provider value={focusKey}>
      <div ref={ref} className="flex flex-1 flex-col overflow-y-auto">
        <Header />
        <div className="px-16">
          <h1 className="mb-4 text-3xl font-bold">Date &amp; Time</h1>

          <section className="space-y-2">
            <p className="text-sm font-medium">Current</p>
            <List>
              <ListRow label="Now" value={now || undefined} />
            </List>
          </section>

          <section className="mt-6 space-y-2">
            <p className="text-sm font-medium">Automatic</p>
            <List>
              <ListRow
                label="Set Automatically"
                accessory="switch"
                checked={status?.ntp_enabled ?? false}
                onSelect={() => status && setNtpEnabled.mutate(!status.ntp_enabled)}
              />
            </List>
            {status?.ntp_enabled && !status.ntp_synchronized && (
              <p className="mt-2 text-sm text-muted-foreground">Waiting to sync -- check the network connection if this doesn't clear.</p>
            )}
          </section>

          <section className="mt-6 space-y-2">
            <p className="text-sm font-medium">Time Zone</p>
            <List>
              <ListRow label="Time Zone" value={status?.timezone} onSelect={() => setTimezoneModalOpen(true)} />
            </List>
          </section>

          {/* Manual entry is only a fallback for when NTP is off -- with NTP on, timedatectl
              set-time fails outright ("Automatic time synchronization is enabled"), so there's
              nothing useful this row could do while that's the case. */}
          {status && !status.ntp_enabled && (
            <section className="mt-6 space-y-2">
              <p className="text-sm font-medium">Set Manually</p>
              <List>
                <TextFieldRow label="Date & Time" value="" placeholder="YYYY-MM-DD HH:MM:SS" onCommit={(value) => setTime.mutate(value)} />
              </List>
              {setTime.isPending && <p className="mt-2 text-sm text-muted-foreground">Setting time...</p>}
              {setTime.isError && <p className="mt-2 text-sm text-destructive">{(setTime.error as Error).message}</p>}
            </section>
          )}

          {error && <p className="mt-6 text-sm text-destructive">{(error as Error).message}</p>}
        </div>
      </div>

      <TimezoneModal
        open={timezoneModalOpen}
        current={status?.timezone}
        onClose={() => setTimezoneModalOpen(false)}
        onSelect={(timezone) => {
          setTimezoneModalOpen(false);
          setTimezone.mutate(timezone);
        }}
      />
    </FocusContext.Provider>
  );
}

function TimezoneModal({
  open,
  current,
  onClose,
  onSelect,
}: {
  open: boolean;
  current: string | undefined;
  onClose: () => void;
  onSelect: (timezone: string) => void;
}) {
  const [query, setQuery] = useState("");
  const { data: timezones = [] } = useTimezones();

  useActionHints(open ? [{ action: "back", label: "Cancel" }] : null);

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    return q ? timezones.filter((tz) => tz.toLowerCase().includes(q)) : timezones;
  }, [timezones, query]);

  return (
    <Modal open={open} onClose={onClose} focusKey="TIMEZONE_MODAL" className="flex max-h-[80vh] w-[32rem] flex-col">
      <div className="flex min-h-0 flex-1 flex-col space-y-4 rounded-lg bg-gray-800 p-6">
        <h2 className="font-space-grotesk text-lg font-bold">Time Zone</h2>
        <SearchField value={query} onChange={setQuery} placeholder="Search time zones" />
        <div className="min-h-0 flex-1 overflow-y-auto">
          <List>
            {filtered.map((tz) => (
              <ListRow key={tz} label={tz} accessory="checkbox" checked={tz === current} onSelect={() => onSelect(tz)} />
            ))}
          </List>
          {filtered.length === 0 && <p className="px-4 py-3 text-sm text-muted-foreground">No time zones match "{query}".</p>}
        </div>
      </div>
    </Modal>
  );
}
