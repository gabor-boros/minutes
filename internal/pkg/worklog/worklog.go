package worklog

// groupEntries ensures to group similar entries, identified by their key.
// If the keys are matching for two entries, those will be merged and their duration will be summed up, notes will be
// concatenated.
func groupEntries(entries []Entry) []Entry {
	entryGroup := map[string]Entry{}

	for _, entry := range entries {
		key := entry.Key()
		storedEntry, isStored := entryGroup[key]

		if !isStored {
			entryGroup[key] = entry
			continue
		}

		storedEntry.BillableDuration += entry.BillableDuration
		storedEntry.UnbillableDuration += entry.UnbillableDuration

		noteSeparator := ""
		if storedEntry.Notes != "" && entry.Notes != storedEntry.Notes {
			if entry.Notes != "" {
				noteSeparator = "; "
			}

			storedEntry.Notes = storedEntry.Notes + noteSeparator + entry.Notes
		}

		entryGroup[key] = storedEntry
	}

	groupedEntries := make([]Entry, 0, len(entryGroup))
	for _, item := range entryGroup {
		groupedEntries = append(groupedEntries, item)
	}

	return groupedEntries
}

// Worklog is the collection of multiple Entries.
type Worklog struct {
	entries []Entry
}

// entryGroup returns those entries that are matching the completeness criteria.
func (w *Worklog) entryGroup(isComplete bool) []Entry {
	var entries []Entry

	for _, entry := range w.entries {
		if entry.IsComplete() == isComplete {
			entries = append(entries, entry)
		}
	}

	return entries
}

// CompleteEntries returns those entries which necessary fields were filled.
func (w *Worklog) CompleteEntries() []Entry {
	return w.entryGroup(true)
}

// IncompleteEntries is the opposite of CompleteEntries.
func (w *Worklog) IncompleteEntries() []Entry {
	return w.entryGroup(false)
}

// NewWorklog creates a worklog from the given set of entries and groups them.
func NewWorklog(entries []Entry) Worklog {
	return Worklog{
		entries: groupEntries(entries),
	}
}
