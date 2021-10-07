package worklog_test

import (
	"testing"

	"github.com/gabor-boros/minutes/internal/pkg/worklog"
	"github.com/stretchr/testify/assert"
)

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
