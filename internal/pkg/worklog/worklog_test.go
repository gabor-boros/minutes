package worklog_test

import (
	"regexp"
	"testing"
	"time"

	"github.com/gabor-boros/minutes/internal/pkg/worklog"
	"github.com/stretchr/testify/assert"
)

var newWorklogBenchResult worklog.Worklog
var completeEntriesBenchResult worklog.Entries
var incompleteEntriesBenchResult worklog.Entries

func benchNewWorklog(b *testing.B, entryCount int) {
	b.StopTimer()

	var entries worklog.Entries

	for i := 0; i != entryCount; i++ {
		entry := getCompleteTestEntry()
		entry.Start.Add(time.Hour * time.Duration(i))
		entries = append(entries, entry)
	}

	b.StartTimer()

	for n := 0; n != b.N; n++ {
		// always store the result to a package level variable
		// so the compiler cannot eliminate the Benchmark itself.
		newWorklogBenchResult = worklog.NewWorklog(entries, &worklog.FilterOpts{})
	}
}

func benchmarkCompleteEntries(b *testing.B, entryCount int) {
	b.StopTimer()

	var entries worklog.Entries

	for i := 0; i != entryCount; i++ {
		entry := getCompleteTestEntry()
		entry.Start.Add(time.Hour * time.Duration(i))
		entries = append(entries, entry)
	}

	wl := worklog.NewWorklog(entries, &worklog.FilterOpts{})

	b.StartTimer()

	for n := 0; n != b.N; n++ {
		// always store the result to a package level variable
		// so the compiler cannot eliminate the Benchmark itself.
		completeEntriesBenchResult = wl.CompleteEntries()
	}
}

func benchmarkIncompleteEntries(b *testing.B, entryCount int) {
	b.StopTimer()

	var entries worklog.Entries

	for i := 0; i != entryCount; i++ {
		entry := getIncompleteTestEntry()
		entry.Start.Add(time.Hour * time.Duration(i))
		entries = append(entries, entry)
	}

	wl := worklog.NewWorklog(entries, &worklog.FilterOpts{})

	b.StartTimer()

	for n := 0; n != b.N; n++ {
		// always store the result to a package level variable
		// so the compiler cannot eliminate the Benchmark itself.
		incompleteEntriesBenchResult = wl.IncompleteEntries()
	}
}

func BenchmarkNewWorklog_10(b *testing.B) {
	benchNewWorklog(b, 10)
	_ = newWorklogBenchResult // Use the result to eliminate linter issues
}

func BenchmarkNewWorklog_1000(b *testing.B) {
	benchNewWorklog(b, 1000)
	_ = newWorklogBenchResult // Use the result to eliminate linter issues
}

func BenchmarkCompleteEntries_10(b *testing.B) {
	benchmarkCompleteEntries(b, 10)
	_ = completeEntriesBenchResult // Use the result to eliminate linter issues
}

func BenchmarkCompleteEntries_1000(b *testing.B) {
	benchmarkCompleteEntries(b, 1000)
	_ = completeEntriesBenchResult // Use the result to eliminate linter issues
}

func BenchmarkIncompleteEntries_10(b *testing.B) {
	benchmarkIncompleteEntries(b, 10)
	_ = incompleteEntriesBenchResult // Use the result to eliminate linter issues
}

func BenchmarkIncompleteEntries_1000(b *testing.B) {
	benchmarkIncompleteEntries(b, 1000)
	_ = incompleteEntriesBenchResult // Use the result to eliminate linter issues
}

func TestWorklogCompleteEntries(t *testing.T) {
	completeEntry := getCompleteTestEntry()

	otherCompleteEntry := getCompleteTestEntry()
	otherCompleteEntry.Notes = "Really"

	incompleteEntry := getCompleteTestEntry()
	incompleteEntry.Task = worklog.IDNameField{}

	wl := worklog.NewWorklog(worklog.Entries{
		completeEntry,
		otherCompleteEntry,
		incompleteEntry,
	}, &worklog.FilterOpts{})

	entry := wl.CompleteEntries()[0]
	assert.Equal(t, "It is a lot easier than expected; Really", entry.Notes)
	assert.Equal(t, worklog.Entries{entry}, wl.CompleteEntries())
}

func TestWorklogIncompleteEntries(t *testing.T) {
	completeEntry := getCompleteTestEntry()

	incompleteEntry := getCompleteTestEntry()
	incompleteEntry.Task = worklog.IDNameField{}

	otherIncompleteEntry := getCompleteTestEntry()
	otherIncompleteEntry.Task = worklog.IDNameField{}
	otherIncompleteEntry.Notes = "Well, not that easy"

	wl := worklog.NewWorklog(worklog.Entries{
		completeEntry,
		incompleteEntry,
		otherIncompleteEntry,
	}, &worklog.FilterOpts{})

	entry := wl.IncompleteEntries()[0]
	assert.Equal(t, "It is a lot easier than expected; Well, not that easy", entry.Notes)
	assert.Equal(t, worklog.Entries{entry}, wl.IncompleteEntries())
}

func TestWorklogFilterEntries(t *testing.T) {
	entry1 := getCompleteTestEntry()
	entry1.Client.Name = "ACME Inc."
	entry1.Project.Name = "redesign website"

	entry2 := getCompleteTestEntry()
	entry2.Client.Name = "ACME Incorporation"
	entry2.Project.Name = "website development"

	entry3 := getCompleteTestEntry()
	entry3.Client.Name = "Other Inc."
	entry3.Project.Name = "redesign website"

	entry4 := getCompleteTestEntry()
	entry4.Client.Name = "Another Inc."
	entry4.Project.Name = "website development"

	filterOpts := &worklog.FilterOpts{
		Client:  regexp.MustCompile(`^ACME Inc\.?(orporation)?$`),
		Project: regexp.MustCompile(`.*(website).*`),
	}

	wl := worklog.NewWorklog(worklog.Entries{
		entry1,
		entry2,
		entry3,
		entry4,
	}, filterOpts)

	assert.ElementsMatch(t, worklog.Entries{entry1, entry2}, wl.CompleteEntries())
}
