"""
Python tests for Posemesh Domain HTTP client.
"""

import os
import sys
import pytest
from datetime import datetime
from typing import Optional

# Add the pkg directory to the path so we can import domain_client
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..', 'pkg'))

from domain_client import (
    DomainClient,
    DomainData,
    DomainDataMetadata,
    DownloadQuery,
    DomainAction,
    UploadDomainData,
    DomainError,
    new_with_app_credential,
    new_with_user_credential,
    ListDomainsQuery,
    ListDomainsResponse,
)


def load_config():
    """Load configuration from environment variables."""
    return {
        'API_URL': os.getenv('API_URL', ''),
        'DDS_URL': os.getenv('DDS_URL', ''),
        'CLIENT_ID': os.getenv('CLIENT_ID', ''),
        'APP_KEY': os.getenv('APP_KEY', ''),
        'APP_SECRET': os.getenv('APP_SECRET', ''),
        'POSEMESH_EMAIL': os.getenv('POSEMESH_EMAIL', ''),
        'POSEMESH_PASSWORD': os.getenv('POSEMESH_PASSWORD', ''),
        'TEST_DOMAIN_SERVER_URL': os.getenv('TEST_DOMAIN_SERVER_URL', ''),
        'TEST_ORGANIZATION': os.getenv('TEST_ORGANIZATION', ''),
        'AUTH_TEST_TOKEN': os.getenv('AUTH_TEST_TOKEN', ''),
    }


config = load_config()


def create_test_domain(client: DomainClient) -> str:
    """
    Create a test domain and upload test data.

    Note: This function requires methods that may not be exposed in Python bindings.
    If createDomain and uploadDomainData are not available, this will need to be
    adapted or the domain_id should be provided via environment variable.
    """
    # For now, we'll need to use an existing domain or skip tests that require
    # domain creation. In a real scenario, you'd call:
    # domain = client.create_domain("test domain " + datetime.now().isoformat(), None, config['TEST_DOMAIN_SERVER_URL'], None)
    # client.upload_domain_data(domain.id, [{"name": "test data", "data_type": "test", "data": b"test data"}])
    # return domain.id

    # For testing purposes, we'll use an environment variable if available
    if config.get('TEST_DOMAIN_SERVER_URL'):
        server_url = config['TEST_DOMAIN_SERVER_URL']
    else:
        server_url = None

    domain = client.create_domain(
        "test domain " + datetime.now().isoformat(),
        None,
        server_url,
        None
    )
    return domain.id


def delete_test_domain(client: DomainClient, domain_id: str):
    """
    Delete a test domain.

    Note: This function requires deleteDomain method which may not be exposed.
    """
    # If deleteDomain is not available, this is a no-op
    # In a real scenario: client.delete_domain(domain_id)
    client.delete_domain(domain_id)


# Pytest fixtures for shared test resources

@pytest.fixture(scope="module")
def user_client():
    """Create a user credential client for tests."""
    if not config['POSEMESH_EMAIL'] or not config['POSEMESH_PASSWORD']:
        pytest.skip("POSEMESH_EMAIL and POSEMESH_PASSWORD environment variables not set")

    client = new_with_user_credential(
        config['API_URL'],
        config['DDS_URL'],
        config['CLIENT_ID'],
        config['POSEMESH_EMAIL'],
        config['POSEMESH_PASSWORD'],
        True
    )
    yield client
    # Cleanup handled by Python's garbage collector


@pytest.fixture(scope="module")
def test_domain_id(user_client):
    """Create or get a test domain ID and delete it after all tests are done."""
    domain_id = create_test_domain(user_client)
    # Create one DomainData in the test domain for downstream tests
    data = [
        UploadDomainData(
            action=DomainAction.CREATE(
                "test_data_1",
                "test"
            ),
            data=bytes([1, 2, 3])
        )
    ]
    user_client.upload_domain_data(domain_id, data)
    try:
        yield domain_id
    finally:
        # Make sure to delete the test domain after all tests, regardless of test pass/fail
        try:
            delete_test_domain(user_client, domain_id)
        except Exception:
            # Ignore errors on deletion to avoid masking test failures
            pass


@pytest.fixture(scope="module")
def app_client():
    """Create an app credential client for tests."""
    if not config['APP_KEY'] or not config['APP_SECRET']:
        pytest.skip("APP_KEY and APP_SECRET environment variables not set")

    client = new_with_app_credential(
        config['API_URL'],
        config['DDS_URL'],
        config['CLIENT_ID'],
        config['APP_KEY'],
        config['APP_SECRET']
    )
    yield client
    # Cleanup handled by Python's garbage collector


# Test classes

class TestAppCredential:
    """Tests for app credential authentication."""

    def test_invalid_app_credential(self):
        """Test that invalid app credentials raise an error."""
        with pytest.raises(DomainError):
            new_with_app_credential(
                config['API_URL'],
                config['DDS_URL'],
                config['CLIENT_ID'],
                "invalid-app-key",
                "invalid-app-secret"
            )

    def test_download_domain_data_with_app_credential(self, app_client, test_domain_id):
        """Test downloading domain data with app credential."""
        query = DownloadQuery(ids=[], name=None, data_type="test")
        data = app_client.download_domain_data(test_domain_id, query)

        assert data is not None
        assert isinstance(data, list)
        assert len(data) > 0

        for item in data:
            assert isinstance(item, DomainData)
            assert len(item.data) > 0
            assert item.metadata.data_type == "test"
            assert item.metadata.id is not None
            assert item.metadata.name is not None
            assert item.metadata.size > 0
            assert item.metadata.created_at is not None
            assert item.metadata.updated_at is not None

    def test_download_domain_data_by_id(self, app_client, test_domain_id):
        """Test downloading a specific domain data by id."""
        # First, get metadata to find an ID
        query = DownloadQuery(ids=[], name=None, data_type="test")
        data = app_client.download_domain_data(test_domain_id, query)

        if len(data) == 0:
            pytest.skip("No domain data found to test download by ID")

        # Note: downloadDomainDataById may not be exposed in Python bindings
        # If it is, we would test it here:
        # data_id = data[0].metadata.id
        # bytes_data = app_client.download_domain_data_by_id(test_domain_id, data_id)
        # assert bytes_data is not None
        # assert len(bytes_data) > 0

        # For now, we verify we can get data with the ID in the query
        data_id = data[0].metadata.id
        query_by_id = DownloadQuery(ids=[data_id], name=None, data_type=None)
        data_by_id = app_client.download_domain_data(test_domain_id, query_by_id)

        assert len(data_by_id) > 0
        assert data_by_id[0].metadata.id == data_id

    def test_download_metadata(self, app_client, test_domain_id):
        """Test downloading metadata for domain data."""
        query = DownloadQuery(ids=[], name=None, data_type="test")
        data = app_client.download_domain_data(test_domain_id, query)

        assert data is not None
        assert isinstance(data, list)
        assert len(data) > 0

        for item in data:
            assert item.metadata.id is not None
            assert item.metadata.name is not None
            assert item.metadata.data_type == "test"
            assert item.metadata.size > 0
            assert item.metadata.created_at is not None
            assert item.metadata.updated_at is not None

    def test_list_domains(self, app_client):
        """Test listing domains."""
        query = ListDomainsQuery(org="own", portal_id=None, portal_short_id=None)
        res = app_client.list_domains(query)
        assert res is not None
        assert isinstance(res, ListDomainsResponse)
        assert len(res.domains) > 0
        for domain in res.domains:
            assert domain.id is not None
            assert domain.name is not None

class TestUserCredential:
    """Tests for user credential authentication."""

    def test_download_domain_data_with_user_credential(self, user_client, test_domain_id):
        """Test downloading domain data with user credential."""
        query = DownloadQuery(ids=[], name=None, data_type="test")
        data_list = user_client.download_domain_data(test_domain_id, query)

        assert isinstance(data_list, list)
        assert len(data_list) > 0

        for item in data_list:
            assert item.metadata.id is not None
            assert item.metadata.name is not None
            assert item.metadata.data_type == "test"
            assert item.metadata.size > 0
            assert item.metadata.created_at is not None
            assert item.metadata.updated_at is not None
            assert len(item.data) > 0

    def test_download_domain_data_with_query_by_ids(self, user_client, test_domain_id):
        """Test downloading domain data with specific IDs."""
        # First get all test data
        query = DownloadQuery(ids=[], name=None, data_type="test")
        all_data = user_client.download_domain_data(test_domain_id, query)

        if len(all_data) == 0:
            pytest.skip("No test data available")

        # Then query by specific IDs
        test_ids = [item.metadata.id for item in all_data[:2]]  # Get first 2 IDs
        query_by_ids = DownloadQuery(ids=test_ids, name=None, data_type=None)
        filtered_data = user_client.download_domain_data(test_domain_id, query_by_ids)

        assert len(filtered_data) == len(test_ids)
        assert all(item.metadata.id in test_ids for item in filtered_data)

    def test_download_domain_data_with_query_by_name(self, user_client, test_domain_id):
        """Test downloading domain data with name filter."""
        # First get all test data to find a name
        query = DownloadQuery(ids=[], name=None, data_type="test")
        all_data = user_client.download_domain_data(test_domain_id, query)

        if len(all_data) == 0:
            pytest.skip("No test data available")

        # Query by name
        test_name = all_data[0].metadata.name
        query_by_name = DownloadQuery(ids=[], name=test_name, data_type=None)
        filtered_data = user_client.download_domain_data(test_domain_id, query_by_name)

        assert len(filtered_data) > 0
        assert all(item.metadata.name == test_name for item in filtered_data)


@pytest.mark.skipif(
    not config.get('AUTH_TEST_TOKEN') or config['AUTH_TEST_TOKEN'] == '',
    reason="AUTH_TEST_TOKEN environment variable not set"
)
class TestOIDCAccessToken:
    """Tests for OIDC access token authentication."""

    @pytest.fixture(scope="class")
    def base_client(self):
        """Create a base client for OIDC tests."""
        return DomainClient(
            config['API_URL'],
            config['DDS_URL'],
            config['CLIENT_ID']
        )

    @pytest.fixture(scope="class")
    def oidc_client(self, base_client):
        """Create a client with OIDC access token."""
        return base_client.with_oidc_access_token(config['AUTH_TEST_TOKEN'])

    def test_download_domain_data_with_oidc_token(self, oidc_client, test_domain_id):
        """Test downloading domain data with OIDC access token."""
        query = DownloadQuery(ids=[], name=None, data_type="test")
        data = oidc_client.download_domain_data(test_domain_id, query)

        assert len(data) > 0
        for item in data:
            assert len(item.data) > 0
            assert item.metadata.data_type == "test"
            assert item.metadata.id is not None
            assert item.metadata.name is not None
            assert item.metadata.size > 0

    def test_invalid_oidc_token(self, base_client, test_domain_id):
        """Test that invalid OIDC access token raises an error."""
        invalid_client = base_client.with_oidc_access_token("invalid_token")
        query = DownloadQuery(ids=[], name=None, data_type="test")

        with pytest.raises(DomainError):
            invalid_client.download_domain_data(test_domain_id, query)

