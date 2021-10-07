package worklog_test

import (
	"testing"
	"time"

	"github.com/gabor-boros/minutes/internal/pkg/worklog"
	"github.com/stretchr/testify/assert"
)

var worklogBenchResult worklog.Worklog

func benchmarkNewWorklog(b *testing.B, entryCount int) {
	b.StopTimer()

	var entries []worklog.Entry

	for i := 0; i != entryCount; i++ {
		entry := getTestEntry()
		entry.Start.Add(time.Hour * time.Duration(i))
		entries = append(entries, entry)
	}

	b.StartTimer()

	var result worklog.Worklog
	for n := 0; n != b.N; n++ {
		result = worklog.NewWorklog(entries)
	}

	// always store the result to a package level variable
	// so the compiler cannot eliminate the Benchmark itself.
	worklogBenchResult = result
}

func BenchmarkNewWorklog_5(b *testing.B) {
	benchmarkNewWorklog(b, 5)
}

func BenchmarkNewWorklog_10(b *testing.B) {
	benchmarkNewWorklog(b, 10)
}

func BenchmarkNewWorklog_50(b *testing.B) {
	benchmarkNewWorklog(b, 50)
}

func BenchmarkNewWorklog_100(b *testing.B) {
	benchmarkNewWorklog(b, 100)
}

func TestWorklogCompleteEntries(t *testing.T) {
	completeEntry := getTestEntry()

	otherCompleteEntry := getTestEntry()
	otherCompleteEntry.Notes = "Really"

	incompleteEntry := getTestEntry()
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
	completeEntry := getTestEntry()

	incompleteEntry := getTestEntry()
	incompleteEntry.Task = worklog.IDNameField{}

	otherIncompleteEntry := getTestEntry()
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
