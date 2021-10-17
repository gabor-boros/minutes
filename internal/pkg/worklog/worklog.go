package worklog

// Worklog is the collection of multiple Entries.
type Worklog struct {
	completeEntries   []Entry
	incompleteEntries []Entry
}

// CompleteEntries returns those entries which necessary fields were filled.
func (w *Worklog) CompleteEntries() []Entry {
	return w.completeEntries
}

// IncompleteEntries is the opposite of CompleteEntries.
func (w *Worklog) IncompleteEntries() []Entry {
	return w.incompleteEntries
}

// NewWorklog creates a worklog from the given set of entries and merges them.
func NewWorklog(entries []Entry) Worklog {
	worklog := Worklog{}
	mergedEntries := map[string]Entry{}

	for _, entry := range entries {
		key := entry.Key()
		storedEntry, isStored := mergedEntries[key]

		if !isStored {
			mergedEntries[key] = entry
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

		mergedEntries[key] = storedEntry
	}

	for _, entry := range mergedEntries {
		if entry.IsComplete() {
			worklog.completeEntries = append(worklog.completeEntries, entry)
		} else {
			worklog.incompleteEntries = append(worklog.incompleteEntries, entry)
		}
	}

	return worklog
}
