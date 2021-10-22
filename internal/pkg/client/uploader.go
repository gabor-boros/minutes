package client

import (
	"context"
	"errors"

	"github.com/gabor-boros/minutes/internal/pkg/worklog"
	"github.com/jedib0t/go-pretty/v6/progress"
)

var (
	// ErrUploadEntries wraps the error when upload failed.
	ErrUploadEntries = errors.New("failed to upload entries")
)

// UploadOpts specifies the only options for the Uploader. In contrast to the
// BaseClientOpts, these options shall not be extended or overridden.
type UploadOpts struct {
	// RoundToClosestMinute indicates to round the billed and unbilled duration
	// separately to the closest minute.
	// If the elapsed time is 30 seconds or more, the closest minute is the
	// next minute, otherwise the previous one. In case the previous minute is
	// 0 (zero), then 0 (zero) will be used for the billed and/or unbilled
	// duration.
	RoundToClosestMinute bool
	// TreatDurationAsBilled indicates to use every time spent as billed.
	TreatDurationAsBilled bool
	// CreateMissingResources indicates the need of resource creation if the
	// resource is missing.
	// In the case of some Uploader, the resources must exist to be able to
	// use them by their ID or name.
	CreateMissingResources bool
	// User represents the user in which name the time log will be uploaded.
	User string
	// ProgressWriter represents a writer that tracks the upload progress.
	// In case the ProgressWriter is nil, that means the upload progress should
	// not be tracked, hence, that's not an error.
	ProgressWriter progress.Writer
}

// Uploader specifies the functions used to upload worklog entries.
type Uploader interface {
	// UploadEntries to a given target.
	// If the upload resulted in an error, the upload will stop and an error
	// will return.
	UploadEntries(ctx context.Context, entries worklog.Entries, errChan chan error, opts *UploadOpts)
}

// DefaultUploader defines helper function to make entry upload easier
type DefaultUploader struct{}

// StartTracking creates a progress tracker, appends to the progress writer, then
// returns the appended writer for later use.
func (u *DefaultUploader) StartTracking(entry worklog.Entry, writer progress.Writer) *progress.Tracker {
	var tracker *progress.Tracker

	if writer != nil {
		tracker = &progress.Tracker{
			Message: entry.Summary,
			Total:   1,
			Units:   progress.UnitsDefault,
		}

		writer.AppendTracker(tracker)
	}

	return tracker
}

func (u *DefaultUploader) StopTracking(tracker *progress.Tracker, err error) {
	if tracker == nil {
		return
	}

	if err == nil {
		tracker.MarkAsDone()
	} else {
		tracker.MarkAsErrored()
	}
}
