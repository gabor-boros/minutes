package clockify_test

import (
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"net/http/httptest"
	"testing"
	"time"

	"github.com/gabor-boros/minutes/internal/pkg/client"
	"github.com/gabor-boros/minutes/internal/pkg/client/clockify"
	"github.com/gabor-boros/minutes/internal/pkg/worklog"
	"github.com/stretchr/testify/require"
)

type mockServerOpts struct {
	Path           string
	Method         string
	StatusCode     int
	Token          string
	TokenHeader    string
	ResponseData   *[]clockify.FetchEntry
	RemainingCalls *int
}

func mockServer(t *testing.T, e *mockServerOpts) *httptest.Server {
	return httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		require.Equal(t, e.Method, r.Method, "API call methods are not matching")
		require.Equal(t, e.Path, r.URL.Path, "API call URLs are not matching")

		if *e.RemainingCalls == 0 {
			w.WriteHeader(http.StatusOK)
			_ = json.NewEncoder(w).Encode(&[]clockify.FetchEntry{})
			return
		}

		if e.Token != "" {
			headerValue := r.Header.Get(e.TokenHeader)
			require.Equal(t, e.Token, headerValue, "API call auth token mismatch")
		}

		if e.ResponseData != nil {
			err := json.NewEncoder(w).Encode(e.ResponseData)
			require.Nil(t, err, "cannot encode response data")
		}

		*e.RemainingCalls--
		w.WriteHeader(e.StatusCode)
	}))
}

func newMockServer(t *testing.T, opts *mockServerOpts) *httptest.Server {
	mockServer := mockServer(t, opts)
	require.NotNil(t, mockServer, "cannot create mock server")
	return mockServer
}

func TestClockifyClient_FetchEntries(t *testing.T) {
	start := time.Date(2021, 10, 2, 0, 0, 0, 0, time.UTC)
	end := time.Date(2021, 10, 2, 23, 59, 59, 0, time.UTC)
	remainingCalls := 1

	expectedEntries := worklog.Entries{
		{
			Client: worklog.IDNameField{
				ID:   "456",
				Name: "My Awesome Company",
			},
			Project: worklog.IDNameField{
				ID:   "123",
				Name: "MARVEL-101",
			},
			Task: worklog.IDNameField{
				ID:   "789",
				Name: "Meet with Iron Man",
			},
			Summary:            "Meet with Iron Man",
			Notes:              "Have a coffee with Tony",
			Start:              start,
			BillableDuration:   end.Sub(start),
			UnbillableDuration: 0,
		},
		{
			Client: worklog.IDNameField{
				ID:   "456",
				Name: "My Awesome Company",
			},
			Project: worklog.IDNameField{
				ID:   "123",
				Name: "MARVEL-101",
			},
			Task: worklog.IDNameField{
				ID:   "789",
				Name: "Meet with Iron Man",
			},
			Summary:            "Meet with Iron Man",
			Notes:              "Go back for my wallet",
			Start:              start,
			BillableDuration:   0,
			UnbillableDuration: end.Sub(start),
		},
	}

	mockServer := newMockServer(t, &mockServerOpts{
		Path:           fmt.Sprintf(clockify.PathWorklog, "marvel-studios", "steve-rogers"),
		Method:         http.MethodGet,
		StatusCode:     http.StatusOK,
		Token:          "t-o-k-e-n",
		TokenHeader:    "X-Api-Key",
		RemainingCalls: &remainingCalls,
		ResponseData: &[]clockify.FetchEntry{
			{
				Description: "Have a coffee with Tony",
				Billable:    true,
				Project: clockify.Project{
					IDNameField: worklog.IDNameField{
						ID:   "123",
						Name: "MARVEL-101",
					},
					ClientID:   "456",
					ClientName: "My Awesome Company",
				},
				TimeInterval: clockify.Interval{
					Start: start,
					End:   end,
				},
				Task: worklog.IDNameField{
					ID:   "789",
					Name: "Meet with Iron Man",
				},
				Tags: []worklog.IDNameField{
					{
						ID:   "1234",
						Name: "Coffee",
					},
					{
						ID:   "5678",
						Name: "Meeting",
					},
					{
						ID:   "9876",
						Name: "TASK-1234",
					},
				},
			},
			{
				Description: "Go back for my wallet",
				Billable:    false,
				Project: clockify.Project{
					IDNameField: worklog.IDNameField{
						ID:   "123",
						Name: "MARVEL-101",
					},
					ClientID:   "456",
					ClientName: "My Awesome Company",
				},
				TimeInterval: clockify.Interval{
					Start: start,
					End:   end,
				},
				Task: worklog.IDNameField{
					ID:   "789",
					Name: "Meet with Iron Man",
				},
				Tags: []worklog.IDNameField{
					{
						ID:   "1234",
						Name: "Coffee",
					},
					{
						ID:   "5678",
						Name: "Meeting",
					},
					{
						ID:   "9876",
						Name: "TASK-1234",
					},
					{
						ID:   "5432",
						Name: "TASK-5678",
					},
				},
			},
		},
	})
	defer mockServer.Close()

	clockifyClient, err := clockify.NewFetcher(&clockify.ClientOpts{
		TokenAuth: client.TokenAuth{
			Header: "X-Api-Key",
			Token:  "t-o-k-e-n",
		},
		BaseURL:   mockServer.URL,
		Workspace: "marvel-studios",
	})

	require.Nil(t, err)

	entries, err := clockifyClient.FetchEntries(context.Background(), &client.FetchOpts{
		User:  "steve-rogers",
		Start: start,
		End:   end,
	})

	require.Nil(t, err, "cannot fetch entries")
	require.ElementsMatch(t, expectedEntries, entries, "fetched entries are not matching")
}

func TestClockifyClient_FetchEntries_TasksAsTags(t *testing.T) {
	start := time.Date(2021, 10, 2, 0, 0, 0, 0, time.UTC)
	end := time.Date(2021, 10, 2, 23, 59, 59, 0, time.UTC)
	remainingCalls := 1

	expectedEntries := worklog.Entries{
		{
			Client: worklog.IDNameField{
				ID:   "456",
				Name: "My Awesome Company",
			},
			Project: worklog.IDNameField{
				ID:   "123",
				Name: "MARVEL-101",
			},
			Task: worklog.IDNameField{
				ID:   "9876",
				Name: "TASK-1234",
			},
			Summary:            "Have a coffee with Tony",
			Notes:              "Have a coffee with Tony",
			Start:              start,
			BillableDuration:   end.Sub(start),
			UnbillableDuration: 0,
		},
		{
			Client: worklog.IDNameField{
				ID:   "456",
				Name: "My Awesome Company",
			},
			Project: worklog.IDNameField{
				ID:   "123",
				Name: "MARVEL-101",
			},
			Task: worklog.IDNameField{
				ID:   "9876",
				Name: "TASK-1234",
			},
			Summary:            "Go back for my wallet",
			Notes:              "Go back for my wallet",
			Start:              start,
			BillableDuration:   0,
			UnbillableDuration: end.Sub(start) / 2,
		},
		{
			Client: worklog.IDNameField{
				ID:   "456",
				Name: "My Awesome Company",
			},
			Project: worklog.IDNameField{
				ID:   "123",
				Name: "MARVEL-101",
			},
			Task: worklog.IDNameField{
				ID:   "5432",
				Name: "TASK-5678",
			},
			Summary:            "Go back for my wallet",
			Notes:              "Go back for my wallet",
			Start:              start,
			BillableDuration:   0,
			UnbillableDuration: end.Sub(start) / 2,
		},
	}

	mockServer := newMockServer(t, &mockServerOpts{
		Path:           fmt.Sprintf(clockify.PathWorklog, "marvel-studios", "steve-rogers"),
		Method:         http.MethodGet,
		StatusCode:     http.StatusOK,
		Token:          "t-o-k-e-n",
		TokenHeader:    "X-Api-Key",
		RemainingCalls: &remainingCalls,
		ResponseData: &[]clockify.FetchEntry{
			{
				Description: "Have a coffee with Tony",
				Billable:    true,
				Project: clockify.Project{
					IDNameField: worklog.IDNameField{
						ID:   "123",
						Name: "MARVEL-101",
					},
					ClientID:   "456",
					ClientName: "My Awesome Company",
				},
				TimeInterval: clockify.Interval{
					Start: start,
					End:   end,
				},
				Task: worklog.IDNameField{},
				Tags: []worklog.IDNameField{
					{
						ID:   "1234",
						Name: "Coffee",
					},
					{
						ID:   "5678",
						Name: "Meeting",
					},
					{
						ID:   "9876",
						Name: "TASK-1234",
					},
				},
			},
			{
				Description: "Go back for my wallet",
				Billable:    false,
				Project: clockify.Project{
					IDNameField: worklog.IDNameField{
						ID:   "123",
						Name: "MARVEL-101",
					},
					ClientID:   "456",
					ClientName: "My Awesome Company",
				},
				TimeInterval: clockify.Interval{
					Start: start,
					End:   end,
				},
				Task: worklog.IDNameField{},
				Tags: []worklog.IDNameField{
					{
						ID:   "1234",
						Name: "Coffee",
					},
					{
						ID:   "5678",
						Name: "Meeting",
					},
					{
						ID:   "9876",
						Name: "TASK-1234",
					},
					{
						ID:   "5432",
						Name: "TASK-5678",
					},
				},
			},
		},
	})
	defer mockServer.Close()

	clockifyClient, err := clockify.NewFetcher(&clockify.ClientOpts{
		BaseClientOpts: client.BaseClientOpts{
			TagsAsTasks:      true,
			TagsAsTasksRegex: `^TASK\-\d+$`,
		},
		TokenAuth: client.TokenAuth{
			Header: "X-Api-Key",
			Token:  "t-o-k-e-n",
		},
		BaseURL:   mockServer.URL,
		Workspace: "marvel-studios",
	})

	require.Nil(t, err)

	entries, err := clockifyClient.FetchEntries(context.Background(), &client.FetchOpts{
		User:  "steve-rogers",
		Start: start,
		End:   end,
	})

	require.Nil(t, err, "cannot fetch entries")
	require.ElementsMatch(t, expectedEntries, entries, "fetched entries are not matching")
}
