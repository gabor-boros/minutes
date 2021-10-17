package worklog_test

import (
	"testing"
	"time"

	"github.com/gabor-boros/minutes/internal/pkg/worklog"
	"github.com/stretchr/testify/assert"
)

var newWorklogBenchResult worklog.Worklog
var completeEntriesBenchResult []worklog.Entry
var incompleteEntriesBenchResult []worklog.Entry

func benchNewWorklog(b *testing.B, entryCount int) {
	b.StopTimer()

	var entries []worklog.Entry

	for i := 0; i != entryCount; i++ {
		entry := getCompleteTestEntry()
		entry.Start.Add(time.Hour * time.Duration(i))
		entries = append(entries, entry)
	}

	b.StartTimer()

	for n := 0; n != b.N; n++ {
		// always store the result to a package level variable
		// so the compiler cannot eliminate the Benchmark itself.
		newWorklogBenchResult = worklog.NewWorklog(entries)
	}
}

func benchmarkCompleteEntries(b *testing.B, entryCount int) {
	b.StopTimer()

	var entries []worklog.Entry

	for i := 0; i != entryCount; i++ {
		entry := getCompleteTestEntry()
		entry.Start.Add(time.Hour * time.Duration(i))
		entries = append(entries, entry)
	}

	wl := worklog.NewWorklog(entries)

	b.StartTimer()

	for n := 0; n != b.N; n++ {
		// always store the result to a package level variable
		// so the compiler cannot eliminate the Benchmark itself.
		completeEntriesBenchResult = wl.CompleteEntries()
	}
}

func benchmarkIncompleteEntries(b *testing.B, entryCount int) {
	b.StopTimer()

	var entries []worklog.Entry

	for i := 0; i != entryCount; i++ {
		entry := getIncompleteTestEntry()
		entry.Start.Add(time.Hour * time.Duration(i))
		entries = append(entries, entry)
	}

	wl := worklog.NewWorklog(entries)

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

	wl := worklog.NewWorklog([]worklog.Entry{
		completeEntry,
		otherCompleteEntry,
		incompleteEntry,
	})

	entry := wl.CompleteEntries()[0]
	assert.Equal(t, "It is a lot easier than expected; Really", entry.Notes)
	assert.Equal(t, []worklog.Entry{entry}, wl.CompleteEntries())
}

func TestWorklogIncompleteEntries(t *testing.T) {
	completeEntry := getCompleteTestEntry()

	incompleteEntry := getCompleteTestEntry()
	incompleteEntry.Task = worklog.IDNameField{}

	otherIncompleteEntry := getCompleteTestEntry()
	otherIncompleteEntry.Task = worklog.IDNameField{}
	otherIncompleteEntry.Notes = "Well, not that easy"

	wl := worklog.NewWorklog([]worklog.Entry{
		completeEntry,
		incompleteEntry,
		otherIncompleteEntry,
	})

	entry := wl.IncompleteEntries()[0]
	assert.Equal(t, "It is a lot easier than expected; Well, not that easy", entry.Notes)
	assert.Equal(t, []worklog.Entry{entry}, wl.IncompleteEntries())
}
