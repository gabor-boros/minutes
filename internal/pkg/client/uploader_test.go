package client_test

import (
	"errors"
	"testing"
	"time"

	"github.com/gabor-boros/minutes/internal/pkg/client"
	"github.com/gabor-boros/minutes/internal/pkg/worklog"
	"github.com/jedib0t/go-pretty/v6/progress"
	"github.com/stretchr/testify/require"
)

func getTestEntry() worklog.Entry {
	start := time.Date(2021, 10, 2, 5, 0, 0, 0, time.UTC)
	end := start.Add(time.Hour * 2)

	return worklog.Entry{
		Client: worklog.IDNameField{
			ID:   "client-id",
			Name: "My Awesome Company",
		},
		Project: worklog.IDNameField{
			ID:   "project-id",
			Name: "Internal projects",
		},
		Task: worklog.IDNameField{
			ID:   "task-id",
			Name: "TASK-0123",
		},
		Summary:            "Write worklog transfer CLI tool",
		Notes:              "It is a lot easier than expected",
		Start:              start,
		BillableDuration:   end.Sub(start),
		UnbillableDuration: 0,
	}
}

func TestDefaultUploader_StartTracking(t *testing.T) {
	entry := getTestEntry()
	progressWriter := progress.NewWriter()

	uploader := client.DefaultUploader{}
	tracker := uploader.StartTracking(entry, progressWriter)

	require.NotNil(t, tracker)
}

func TestDefaultUploader_StartTracking_NoProgressWriter(t *testing.T) {
	entry := getTestEntry()

	uploader := client.DefaultUploader{}
	tracker := uploader.StartTracking(entry, nil)

	require.Nil(t, tracker)
}

func TestDefaultUploader_StopTracking_Success(t *testing.T) {
	entry := getTestEntry()
	progressWriter := progress.NewWriter()

	uploader := client.DefaultUploader{}

	tracker := uploader.StartTracking(entry, progressWriter)
	require.NotNil(t, tracker)

	uploader.StopTracking(tracker, nil)
	require.True(t, tracker.IsDone())
	require.False(t, tracker.IsErrored())
}

func TestDefaultUploader_StopTracking_Failure(t *testing.T) {
	entry := getTestEntry()
	progressWriter := progress.NewWriter()

	uploader := client.DefaultUploader{}

	tracker := uploader.StartTracking(entry, progressWriter)
	require.NotNil(t, tracker)

	uploader.StopTracking(tracker, errors.New("some error"))
	require.True(t, tracker.IsDone())
	require.True(t, tracker.IsErrored())
}

func TestDefaultUploader_StopTracking_NoTracker(t *testing.T) {
	entry := getTestEntry()

	uploader := client.DefaultUploader{}

	tracker := uploader.StartTracking(entry, nil)
	require.Nil(t, tracker)

	uploader.StopTracking(tracker, nil)
}
