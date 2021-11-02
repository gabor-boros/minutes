package harvest_test

import (
	"context"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"net/url"
	"testing"
	"time"

	"github.com/gabor-boros/minutes/internal/pkg/client"
	"github.com/gabor-boros/minutes/internal/pkg/client/harvest"
	"github.com/gabor-boros/minutes/internal/pkg/utils"
	"github.com/gabor-boros/minutes/internal/pkg/worklog"

	"github.com/stretchr/testify/require"
)

type mockServerOpts struct {
	Path         string
	QueryParams  url.Values
	Method       string
	StatusCode   int
	Token        string
	TokenHeader  string
	ResponseData *harvest.FetchResponse
}

func mockServer(t *testing.T, e *mockServerOpts) *httptest.Server {
	return httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		require.Equal(t, e.Method, r.Method, "API call methods are not matching")
		require.Equal(t, e.Path, r.URL.Path, "API call URLs are not matching")
		require.Equal(t, e.QueryParams, r.URL.Query())

		if e.Token != "" {
			headerValue := r.Header.Get(e.TokenHeader)
			require.Equal(t, e.Token, headerValue, "API call auth token mismatch")
		}

		if e.ResponseData != nil {
			err := json.NewEncoder(w).Encode(e.ResponseData)
			require.Nil(t, err, "cannot encode response data")
		}

		w.WriteHeader(e.StatusCode)
	}))
}

func newMockServer(t *testing.T, opts *mockServerOpts) *httptest.Server {
	mockServer := mockServer(t, opts)
	require.NotNil(t, mockServer, "cannot create mock server")
	return mockServer
}

func TestFetchEntry_Start(t *testing.T) {
	expectedStart := time.Date(2021, 9, 30, 23, 59, 59, 0, time.UTC)

	entry := harvest.FetchEntry{
		SpentDate: "2021-09-30",
		CreatedAt: time.Date(2021, 10, 1, 23, 59, 59, 0, time.UTC),
	}

	startDate, err := entry.Start()

	require.Nil(t, err)
	require.Equal(t, expectedStart, startDate)
}

func TestHarvestClient_FetchEntries(t *testing.T) {
	start := time.Date(2021, 10, 2, 0, 0, 0, 0, time.UTC)
	end := time.Date(2021, 10, 2, 23, 59, 59, 0, time.UTC)

	expectedEntries := worklog.Entries{
		{
			Client: worklog.IDNameField{
				ID:   "1",
				Name: "My Awesome Company",
			},
			Project: worklog.IDNameField{
				ID:   "11",
				Name: "MARVEL",
			},
			Task: worklog.IDNameField{
				ID:   "111",
				Name: "CPT-2014",
			},
			Summary:            "I met with The Winter Soldier",
			Notes:              "I met with The Winter Soldier",
			Start:              start,
			BillableDuration:   time.Hour * 2,
			UnbillableDuration: 0,
		},
		{
			Client: worklog.IDNameField{
				ID:   "1",
				Name: "My Awesome Company",
			},
			Project: worklog.IDNameField{
				ID:   "11",
				Name: "MARVEL",
			},
			Task: worklog.IDNameField{
				ID:   "111",
				Name: "CPT-2014",
			},
			Summary:            "I helped him to get back on track",
			Notes:              "I helped him to get back on track",
			Start:              start,
			BillableDuration:   0,
			UnbillableDuration: time.Hour * 3,
		},
	}

	mockServer := newMockServer(t, &mockServerOpts{
		Path: harvest.PathWorklog,
		QueryParams: url.Values{
			"page":       {"1"},
			"per_page":   {"50"},
			"from":       {utils.DateFormatRFC3339UTC.Format(start)},
			"to":         {utils.DateFormatRFC3339UTC.Format(end)},
			"user_id":    {"987654321"},
			"is_running": {"false"},
			"user_agent": {"github.com/gabor-boros/minutes"},
		},
		Method:      http.MethodGet,
		StatusCode:  http.StatusOK,
		Token:       "Bearer t-o-k-e-n",
		TokenHeader: "Authorization",
		ResponseData: &harvest.FetchResponse{
			TimeEntries: []harvest.FetchEntry{
				{
					Client: worklog.IntIDNameField{
						ID:   1,
						Name: "My Awesome Company",
					},
					Project: worklog.IntIDNameField{
						ID:   11,
						Name: "MARVEL",
					},
					Task: worklog.IntIDNameField{
						ID:   111,
						Name: "CPT-2014",
					},
					Notes:     "I met with The Winter Soldier",
					SpentDate: utils.DateFormatISO8601.Format(start),
					Hours:     2.0,
					CreatedAt: start,
					Billable:  true,
					IsRunning: false,
				},
				{
					Client: worklog.IntIDNameField{
						ID:   1,
						Name: "My Awesome Company",
					},
					Project: worklog.IntIDNameField{
						ID:   11,
						Name: "MARVEL",
					},
					Task: worklog.IntIDNameField{
						ID:   111,
						Name: "CPT-2014",
					},
					Notes:     "I helped him to get back on track",
					SpentDate: utils.DateFormatISO8601.Format(start),
					Hours:     3.0,
					CreatedAt: start,
					Billable:  false,
					IsRunning: false,
				},
			},
			PerPage:      50,
			TotalEntries: 2,
		},
	})
	defer mockServer.Close()

	harvestClient, err := harvest.NewFetcher(&harvest.ClientOpts{
		BaseClientOpts: client.BaseClientOpts{
			Timeout: client.DefaultRequestTimeout,
		},
		TokenAuth: client.TokenAuth{
			Header:    "Authorization",
			TokenName: "Bearer",
			Token:     "t-o-k-e-n",
		},
		BaseURL: mockServer.URL,
		Account: 123456789,
	})
	require.Nil(t, err)

	entries, err := harvestClient.FetchEntries(context.Background(), &client.FetchOpts{
		User:  "987654321",
		Start: start,
		End:   end,
	})

	require.Nil(t, err, "cannot fetch entries")
	require.ElementsMatch(t, expectedEntries, entries, "fetched entries are not matching")
}
