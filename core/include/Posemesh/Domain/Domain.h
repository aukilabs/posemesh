/* Warning, this file is autogenerated by cbindgen. Don't modify this manually. */

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

#define UNLIMITED_CAPACITY -1

#define CHUNK_SIZE (7 * 1024)

typedef struct DataConsumer DataConsumer;

typedef struct DatastoreWrapper_RemoteDatastore DatastoreWrapper_RemoteDatastore;

typedef struct DomainCluster DomainCluster;

typedef struct Query Query;

typedef struct ReliableDataProducer ReliableDataProducer;

typedef struct ReliableDataProducer ReliableDataProducer;

typedef struct DomainData {
  const char *domain_id;
  const char *id;
  const char *name;
  const char *data_type;
  const char *properties;
  void *content;
  uintptr_t content_size;
} DomainData;

typedef struct DomainError {
  const char *message;
} DomainError;

typedef void (*FindCallback)(void*, const struct DomainData*, const struct DomainError*);

typedef struct UploadResult {
  const char *id;
  struct DomainError *error;
} UploadResult;

#ifdef __cplusplus
extern "C" {
#endif // __cplusplus

void free_domain_data(struct DomainData *data);

struct Query *create_domain_data_query(const char *const *ids_ptr,
                                       int len,
                                       const char *name_regexp,
                                       const char *data_type_regexp,
                                       const char *const *names,
                                       int names_len,
                                       const char *const *data_types,
                                       int data_types_len);

void free_domain_data_query(struct Query *query);

void free_domain_error(struct DomainError *error);

struct DomainCluster *init_domain_cluster(const char *domain_manager_addr,
                                          const char *name,
                                          const char *const *static_relay_nodes,
                                          uintptr_t static_relay_nodes_len);

void free_domain_cluster(struct DomainCluster *cluster);

struct DatastoreWrapper_RemoteDatastore *init_remote_storage(struct DomainCluster *cluster);

void free_datastore(struct DatastoreWrapper_RemoteDatastore *store);

struct DataConsumer *initialize_data_consumer(struct DatastoreWrapper_RemoteDatastore *store,
                                              const char *domain_id,
                                              struct Query *query,
                                              bool keep_alive,
                                              FindCallback callback,
                                              void *user_data);

void free_data_consumer(struct DataConsumer *consumer);

void free_reliable_data_producer(struct ReliableDataProducer *producer);

struct ReliableDataProducer *initialize_reliable_data_producer(struct DatastoreWrapper_RemoteDatastore *store,
                                                               const char *domain_id);

void free_upload_result(struct UploadResult *result);

const struct UploadResult *upload_domain_data(struct ReliableDataProducer *producer,
                                              const struct DomainData *data);

#ifdef __cplusplus
}  // extern "C"
#endif  // __cplusplus
