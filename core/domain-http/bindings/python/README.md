# Posemesh Domain HTTP Python Tests

This directory contains comprehensive tests for the Python bindings of the `domain-http` package.

## Setup

1. Install dependencies:
```bash
pip install -r requirements.txt
```

2. Set up environment variables (create a `.env.local` file from `.env`)

## Running Tests

Run all tests:
```bash
pytest
```

Run with verbose output:
```bash
pytest -v
```

Run a specific test class:
```bash
pytest test_basic.py::TestPosemeshDomainHTTP::TestAppCredential
```

Run a specific test:
```bash
pytest test_basic.py::TestPosemeshDomainHTTP::TestAppCredential::test_download_domain_data_with_app_credential
```

## Test Structur

- **TestPosemeshDomainHTTP**: Main test class that sets up a test domain
- **TestAppCredential**: Tests for app credential authentication
- **TestUserCredential**: Tests for user credential authentication  
- **TestOIDCAccessToken**: Tests for OIDC access token authentication (requires AUTH_TEST_TOKEN)

## Note on Available Methods

The Python bindings currently expose:
- `new_with_app_credential()` - Create client with app credentials
- `new_with_user_credential()` - Create client with user credentials
- `DomainClient.download_domain_data()` - Download domain data
- `DomainClient.with_oidc_access_token()` - Create client with OIDC token

Some methods available in JavaScript bindings (like `createDomain`, `deleteDomain`, `uploadDomainData`, etc.) may not be exposed in Python bindings yet. Tests that require these methods will be skipped if the functionality is not available.

