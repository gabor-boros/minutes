package worklog

import "regexp"

// FilterOpts represents the worklog creation filtering options.
// When filtering options are set, the entries are matching the regex will be
// part of the worklog, and the rest of them will be dropped. The filtering
// could be part of the fetching process, though that would be less flexible as
// some APIs are not allowing filtering. Also, this way, we can filter results
// using regex.
type FilterOpts struct {
	Client   *regexp.Regexp
	Project  *regexp.Regexp
}

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

// isEntryMatching returns true if the entry matching the filter options.
func isEntryMatching(entry Entry, opts *FilterOpts) bool {
	isClientMatching := opts.Client == nil || opts.Client.MatchString(entry.Client.Name)
	isProjectMatching := opts.Project == nil || opts.Project.MatchString(entry.Project.Name)

	return isClientMatching && isProjectMatching
}

// NewWorklog creates a worklog from the given set of entries and merges them.
func NewWorklog(entries []Entry, opts *FilterOpts) Worklog {
	var filteredEntries []Entry

	worklog := Worklog{}
	mergedEntries := map[string]Entry{}

	for _, entry := range entries {
		if isEntryMatching(entry, opts) {
			filteredEntries = append(filteredEntries, entry)
		}
	}

	for _, entry := range filteredEntries {
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
