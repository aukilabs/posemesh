export const encodeCode = {
  OK: 200,
  Created: 201,
  Accepted: 202,
  BadRequest: 400,
};

export const decodeCode = {
  200: "OK",
  201: "Created",
  202: "Accepted",
  400: "BadRequest",
};

export const encodeStatus = {
  PENDING: 0,
  STARTED: 1,
  DONE: 2,
  FAILED: 3,
  WAITING_FOR_RESOURCE: 4,
};

export const decodeStatus = {
  0: "PENDING",
  1: "STARTED",
  2: "DONE",
  3: "FAILED",
  4: "WAITING_FOR_RESOURCE",
};

export function encodeAny(message) {
  let bb = popByteBuffer();
  _encodeAny(message, bb);
  return toUint8Array(bb);
}

function _encodeAny(message, bb) {
  // optional string type_url = 1;
  let $type_url = message.type_url;
  if ($type_url !== undefined) {
    writeVarint32(bb, 10);
    writeString(bb, $type_url);
  }

  // optional bytes value = 2;
  let $value = message.value;
  if ($value !== undefined) {
    writeVarint32(bb, 18);
    writeVarint32(bb, $value.length), writeBytes(bb, $value);
  }
}

export function decodeAny(binary) {
  return _decodeAny(wrapByteBuffer(binary));
}

function _decodeAny(bb) {
  let message = {};

  end_of_message: while (!isAtEnd(bb)) {
    let tag = readVarint32(bb);

    switch (tag >>> 3) {
      case 0:
        break end_of_message;

      // optional string type_url = 1;
      case 1: {
        message.type_url = readString(bb, readVarint32(bb));
        break;
      }

      // optional bytes value = 2;
      case 2: {
        message.value = readBytes(bb, readVarint32(bb));
        break;
      }

      default:
        skipUnknownField(bb, tag & 7);
    }
  }

  return message;
}

export function encodeJobRequest(message) {
  let bb = popByteBuffer();
  _encodeJobRequest(message, bb);
  return toUint8Array(bb);
}

function _encodeJobRequest(message, bb) {
  // optional string name = 1;
  let $name = message.name;
  if ($name !== undefined) {
    writeVarint32(bb, 10);
    writeString(bb, $name);
  }

  // repeated TaskRequest tasks = 2;
  let array$tasks = message.tasks;
  if (array$tasks !== undefined) {
    for (let value of array$tasks) {
      writeVarint32(bb, 18);
      let nested = popByteBuffer();
      _encodeTaskRequest(value, nested);
      writeVarint32(bb, nested.limit);
      writeByteBuffer(bb, nested);
      pushByteBuffer(nested);
    }
  }

  // required string nonce = 3;
  let $nonce = message.nonce;
  if ($nonce !== undefined) {
    writeVarint32(bb, 26);
    writeString(bb, $nonce);
  }
}

export function decodeJobRequest(binary) {
  return _decodeJobRequest(wrapByteBuffer(binary));
}

function _decodeJobRequest(bb) {
  let message = {};

  end_of_message: while (!isAtEnd(bb)) {
    let tag = readVarint32(bb);

    switch (tag >>> 3) {
      case 0:
        break end_of_message;

      // optional string name = 1;
      case 1: {
        message.name = readString(bb, readVarint32(bb));
        break;
      }

      // repeated TaskRequest tasks = 2;
      case 2: {
        let limit = pushTemporaryLength(bb);
        let values = message.tasks || (message.tasks = []);
        values.push(_decodeTaskRequest(bb));
        bb.limit = limit;
        break;
      }

      // required string nonce = 3;
      case 3: {
        message.nonce = readString(bb, readVarint32(bb));
        break;
      }

      default:
        skipUnknownField(bb, tag & 7);
    }
  }

  if (message.nonce === undefined)
    throw new Error("Missing required field: nonce");

  return message;
}

export function encodeJob(message) {
  let bb = popByteBuffer();
  _encodeJob(message, bb);
  return toUint8Array(bb);
}

function _encodeJob(message, bb) {
  // optional string id = 1;
  let $id = message.id;
  if ($id !== undefined) {
    writeVarint32(bb, 10);
    writeString(bb, $id);
  }

  // optional string name = 2;
  let $name = message.name;
  if ($name !== undefined) {
    writeVarint32(bb, 18);
    writeString(bb, $name);
  }

  // repeated Task tasks = 3;
  let array$tasks = message.tasks;
  if (array$tasks !== undefined) {
    for (let value of array$tasks) {
      writeVarint32(bb, 26);
      let nested = popByteBuffer();
      _encodeTask(value, nested);
      writeVarint32(bb, nested.limit);
      writeByteBuffer(bb, nested);
      pushByteBuffer(nested);
    }
  }
}

export function decodeJob(binary) {
  return _decodeJob(wrapByteBuffer(binary));
}

function _decodeJob(bb) {
  let message = {};

  end_of_message: while (!isAtEnd(bb)) {
    let tag = readVarint32(bb);

    switch (tag >>> 3) {
      case 0:
        break end_of_message;

      // optional string id = 1;
      case 1: {
        message.id = readString(bb, readVarint32(bb));
        break;
      }

      // optional string name = 2;
      case 2: {
        message.name = readString(bb, readVarint32(bb));
        break;
      }

      // repeated Task tasks = 3;
      case 3: {
        let limit = pushTemporaryLength(bb);
        let values = message.tasks || (message.tasks = []);
        values.push(_decodeTask(bb));
        bb.limit = limit;
        break;
      }

      default:
        skipUnknownField(bb, tag & 7);
    }
  }

  return message;
}

export function encodeSubmitJobResponse(message) {
  let bb = popByteBuffer();
  _encodeSubmitJobResponse(message, bb);
  return toUint8Array(bb);
}

function _encodeSubmitJobResponse(message, bb) {
  // optional Code code = 1;
  let $code = message.code;
  if ($code !== undefined) {
    writeVarint32(bb, 8);
    writeVarint32(bb, encodeCode[$code]);
  }

  // optional string job_id = 2;
  let $job_id = message.job_id;
  if ($job_id !== undefined) {
    writeVarint32(bb, 18);
    writeString(bb, $job_id);
  }

  // optional string err_msg = 3;
  let $err_msg = message.err_msg;
  if ($err_msg !== undefined) {
    writeVarint32(bb, 26);
    writeString(bb, $err_msg);
  }
}

export function decodeSubmitJobResponse(binary) {
  return _decodeSubmitJobResponse(wrapByteBuffer(binary));
}

function _decodeSubmitJobResponse(bb) {
  let message = {};

  end_of_message: while (!isAtEnd(bb)) {
    let tag = readVarint32(bb);

    switch (tag >>> 3) {
      case 0:
        break end_of_message;

      // optional Code code = 1;
      case 1: {
        message.code = decodeCode[readVarint32(bb)];
        break;
      }

      // optional string job_id = 2;
      case 2: {
        message.job_id = readString(bb, readVarint32(bb));
        break;
      }

      // optional string err_msg = 3;
      case 3: {
        message.err_msg = readString(bb, readVarint32(bb));
        break;
      }

      default:
        skipUnknownField(bb, tag & 7);
    }
  }

  return message;
}

export function encodeTaskRequest(message) {
  let bb = popByteBuffer();
  _encodeTaskRequest(message, bb);
  return toUint8Array(bb);
}

function _encodeTaskRequest(message, bb) {
  // optional string name = 1;
  let $name = message.name;
  if ($name !== undefined) {
    writeVarint32(bb, 10);
    writeString(bb, $name);
  }

  // optional CapabilityFilters capability_filters = 2;
  let $capability_filters = message.capability_filters;
  if ($capability_filters !== undefined) {
    writeVarint32(bb, 18);
    let nested = popByteBuffer();
    _encodeCapabilityFilters($capability_filters, nested);
    writeVarint32(bb, nested.limit);
    writeByteBuffer(bb, nested);
    pushByteBuffer(nested);
  }

  // optional uint64 max_budget = 3;
  let $max_budget = message.max_budget;
  if ($max_budget !== undefined) {
    writeVarint32(bb, 24);
    writeVarint64(bb, $max_budget);
  }

  // optional string timeout = 4;
  let $timeout = message.timeout;
  if ($timeout !== undefined) {
    writeVarint32(bb, 34);
    writeString(bb, $timeout);
  }

  // repeated string needs = 5;
  let array$needs = message.needs;
  if (array$needs !== undefined) {
    for (let value of array$needs) {
      writeVarint32(bb, 42);
      writeString(bb, value);
    }
  }

  // optional ResourceRecruitment resource_recruitment = 6;
  let $resource_recruitment = message.resource_recruitment;
  if ($resource_recruitment !== undefined) {
    writeVarint32(bb, 50);
    let nested = popByteBuffer();
    _encodeResourceRecruitment($resource_recruitment, nested);
    writeVarint32(bb, nested.limit);
    writeByteBuffer(bb, nested);
    pushByteBuffer(nested);
  }

  // optional string sender = 7;
  let $sender = message.sender;
  if ($sender !== undefined) {
    writeVarint32(bb, 58);
    writeString(bb, $sender);
  }

  // optional string receiver = 8;
  let $receiver = message.receiver;
  if ($receiver !== undefined) {
    writeVarint32(bb, 66);
    writeString(bb, $receiver);
  }

  // optional Any data = 9;
  let $data = message.data;
  if ($data !== undefined) {
    writeVarint32(bb, 74);
    let nested = popByteBuffer();
    _encodeAny($data, nested);
    writeVarint32(bb, nested.limit);
    writeByteBuffer(bb, nested);
    pushByteBuffer(nested);
  }
}

export function decodeTaskRequest(binary) {
  return _decodeTaskRequest(wrapByteBuffer(binary));
}

function _decodeTaskRequest(bb) {
  let message = {};

  end_of_message: while (!isAtEnd(bb)) {
    let tag = readVarint32(bb);

    switch (tag >>> 3) {
      case 0:
        break end_of_message;

      // optional string name = 1;
      case 1: {
        message.name = readString(bb, readVarint32(bb));
        break;
      }

      // optional CapabilityFilters capability_filters = 2;
      case 2: {
        let limit = pushTemporaryLength(bb);
        message.capability_filters = _decodeCapabilityFilters(bb);
        bb.limit = limit;
        break;
      }

      // optional uint64 max_budget = 3;
      case 3: {
        message.max_budget = readVarint64(bb, /* unsigned */ true);
        break;
      }

      // optional string timeout = 4;
      case 4: {
        message.timeout = readString(bb, readVarint32(bb));
        break;
      }

      // repeated string needs = 5;
      case 5: {
        let values = message.needs || (message.needs = []);
        values.push(readString(bb, readVarint32(bb)));
        break;
      }

      // optional ResourceRecruitment resource_recruitment = 6;
      case 6: {
        let limit = pushTemporaryLength(bb);
        message.resource_recruitment = _decodeResourceRecruitment(bb);
        bb.limit = limit;
        break;
      }

      // optional string sender = 7;
      case 7: {
        message.sender = readString(bb, readVarint32(bb));
        break;
      }

      // optional string receiver = 8;
      case 8: {
        message.receiver = readString(bb, readVarint32(bb));
        break;
      }

      // optional Any data = 9;
      case 9: {
        let limit = pushTemporaryLength(bb);
        message.data = _decodeAny(bb);
        bb.limit = limit;
        break;
      }

      default:
        skipUnknownField(bb, tag & 7);
    }
  }

  return message;
}

export function encodeCapabilityFilters(message) {
  let bb = popByteBuffer();
  _encodeCapabilityFilters(message, bb);
  return toUint8Array(bb);
}

function _encodeCapabilityFilters(message, bb) {
  // optional string endpoint = 1;
  let $endpoint = message.endpoint;
  if ($endpoint !== undefined) {
    writeVarint32(bb, 10);
    writeString(bb, $endpoint);
  }

  // optional int32 min_gpu = 2;
  let $min_gpu = message.min_gpu;
  if ($min_gpu !== undefined) {
    writeVarint32(bb, 16);
    writeVarint64(bb, intToLong($min_gpu));
  }

  // optional int32 min_cpu = 3;
  let $min_cpu = message.min_cpu;
  if ($min_cpu !== undefined) {
    writeVarint32(bb, 24);
    writeVarint64(bb, intToLong($min_cpu));
  }
}

export function decodeCapabilityFilters(binary) {
  return _decodeCapabilityFilters(wrapByteBuffer(binary));
}

function _decodeCapabilityFilters(bb) {
  let message = {};

  end_of_message: while (!isAtEnd(bb)) {
    let tag = readVarint32(bb);

    switch (tag >>> 3) {
      case 0:
        break end_of_message;

      // optional string endpoint = 1;
      case 1: {
        message.endpoint = readString(bb, readVarint32(bb));
        break;
      }

      // optional int32 min_gpu = 2;
      case 2: {
        message.min_gpu = readVarint32(bb);
        break;
      }

      // optional int32 min_cpu = 3;
      case 3: {
        message.min_cpu = readVarint32(bb);
        break;
      }

      default:
        skipUnknownField(bb, tag & 7);
    }
  }

  return message;
}

export function encodeResourceRecruitment(message) {
  let bb = popByteBuffer();
  _encodeResourceRecruitment(message, bb);
  return toUint8Array(bb);
}

function _encodeResourceRecruitment(message, bb) {
  // optional RecruitmentPolicy recruitment_policy = 1;
  let $recruitment_policy = message.recruitment_policy;
  if ($recruitment_policy !== undefined) {
    writeVarint32(bb, 10);
    let nested = popByteBuffer();
    _encodeRecruitmentPolicy($recruitment_policy, nested);
    writeVarint32(bb, nested.limit);
    writeByteBuffer(bb, nested);
    pushByteBuffer(nested);
  }

  // optional TerminationPolicy termination_policy = 2;
  let $termination_policy = message.termination_policy;
  if ($termination_policy !== undefined) {
    writeVarint32(bb, 18);
    let nested = popByteBuffer();
    _encodeTerminationPolicy($termination_policy, nested);
    writeVarint32(bb, nested.limit);
    writeByteBuffer(bb, nested);
    pushByteBuffer(nested);
  }
}

export function decodeResourceRecruitment(binary) {
  return _decodeResourceRecruitment(wrapByteBuffer(binary));
}

function _decodeResourceRecruitment(bb) {
  let message = {};

  end_of_message: while (!isAtEnd(bb)) {
    let tag = readVarint32(bb);

    switch (tag >>> 3) {
      case 0:
        break end_of_message;

      // optional RecruitmentPolicy recruitment_policy = 1;
      case 1: {
        let limit = pushTemporaryLength(bb);
        message.recruitment_policy = _decodeRecruitmentPolicy(bb);
        bb.limit = limit;
        break;
      }

      // optional TerminationPolicy termination_policy = 2;
      case 2: {
        let limit = pushTemporaryLength(bb);
        message.termination_policy = _decodeTerminationPolicy(bb);
        bb.limit = limit;
        break;
      }

      default:
        skipUnknownField(bb, tag & 7);
    }
  }

  return message;
}

export function encodeConsumeDataInputV1(message) {
  let bb = popByteBuffer();
  _encodeConsumeDataInputV1(message, bb);
  return toUint8Array(bb);
}

function _encodeConsumeDataInputV1(message, bb) {
  // optional domain_data.Query query = 1;
  let $query = message.query;
  if ($query !== undefined) {
    writeVarint32(bb, 10);
    let nested = popByteBuffer();
    _encodedomain_data.Query($query, nested);
    writeVarint32(bb, nested.limit);
    writeByteBuffer(bb, nested);
    pushByteBuffer(nested);
  }

  // optional bool keep_alive = 2;
  let $keep_alive = message.keep_alive;
  if ($keep_alive !== undefined) {
    writeVarint32(bb, 16);
    writeByte(bb, $keep_alive ? 1 : 0);
  }
}

export function decodeConsumeDataInputV1(binary) {
  return _decodeConsumeDataInputV1(wrapByteBuffer(binary));
}

function _decodeConsumeDataInputV1(bb) {
  let message = {};

  end_of_message: while (!isAtEnd(bb)) {
    let tag = readVarint32(bb);

    switch (tag >>> 3) {
      case 0:
        break end_of_message;

      // optional domain_data.Query query = 1;
      case 1: {
        let limit = pushTemporaryLength(bb);
        message.query = _decodedomain_data.Query(bb);
        bb.limit = limit;
        break;
      }

      // optional bool keep_alive = 2;
      case 2: {
        message.keep_alive = !!readByte(bb);
        break;
      }

      default:
        skipUnknownField(bb, tag & 7);
    }
  }

  return message;
}

export function encodeStoreDataOutputV1(message) {
  let bb = popByteBuffer();
  _encodeStoreDataOutputV1(message, bb);
  return toUint8Array(bb);
}

function _encodeStoreDataOutputV1(message, bb) {
  // repeated string ids = 1;
  let array$ids = message.ids;
  if (array$ids !== undefined) {
    for (let value of array$ids) {
      writeVarint32(bb, 10);
      writeString(bb, value);
    }
  }
}

export function decodeStoreDataOutputV1(binary) {
  return _decodeStoreDataOutputV1(wrapByteBuffer(binary));
}

function _decodeStoreDataOutputV1(bb) {
  let message = {};

  end_of_message: while (!isAtEnd(bb)) {
    let tag = readVarint32(bb);

    switch (tag >>> 3) {
      case 0:
        break end_of_message;

      // repeated string ids = 1;
      case 1: {
        let values = message.ids || (message.ids = []);
        values.push(readString(bb, readVarint32(bb)));
        break;
      }

      default:
        skipUnknownField(bb, tag & 7);
    }
  }

  return message;
}

export function encodeLocalRefinementOutputV1(message) {
  let bb = popByteBuffer();
  _encodeLocalRefinementOutputV1(message, bb);
  return toUint8Array(bb);
}

function _encodeLocalRefinementOutputV1(message, bb) {
  // repeated string result_ids = 1;
  let array$result_ids = message.result_ids;
  if (array$result_ids !== undefined) {
    for (let value of array$result_ids) {
      writeVarint32(bb, 10);
      writeString(bb, value);
    }
  }
}

export function decodeLocalRefinementOutputV1(binary) {
  return _decodeLocalRefinementOutputV1(wrapByteBuffer(binary));
}

function _decodeLocalRefinementOutputV1(bb) {
  let message = {};

  end_of_message: while (!isAtEnd(bb)) {
    let tag = readVarint32(bb);

    switch (tag >>> 3) {
      case 0:
        break end_of_message;

      // repeated string result_ids = 1;
      case 1: {
        let values = message.result_ids || (message.result_ids = []);
        values.push(readString(bb, readVarint32(bb)));
        break;
      }

      default:
        skipUnknownField(bb, tag & 7);
    }
  }

  return message;
}

export function encodeTask(message) {
  let bb = popByteBuffer();
  _encodeTask(message, bb);
  return toUint8Array(bb);
}

function _encodeTask(message, bb) {
  // optional string name = 2;
  let $name = message.name;
  if ($name !== undefined) {
    writeVarint32(bb, 18);
    writeString(bb, $name);
  }

  // optional string receiver = 3;
  let $receiver = message.receiver;
  if ($receiver !== undefined) {
    writeVarint32(bb, 26);
    writeString(bb, $receiver);
  }

  // optional string endpoint = 4;
  let $endpoint = message.endpoint;
  if ($endpoint !== undefined) {
    writeVarint32(bb, 34);
    writeString(bb, $endpoint);
  }

  // optional string access_token = 5;
  let $access_token = message.access_token;
  if ($access_token !== undefined) {
    writeVarint32(bb, 42);
    writeString(bb, $access_token);
  }

  // optional string job_id = 6;
  let $job_id = message.job_id;
  if ($job_id !== undefined) {
    writeVarint32(bb, 50);
    writeString(bb, $job_id);
  }

  // optional string sender = 7;
  let $sender = message.sender;
  if ($sender !== undefined) {
    writeVarint32(bb, 58);
    writeString(bb, $sender);
  }

  // optional Status status = 9;
  let $status = message.status;
  if ($status !== undefined) {
    writeVarint32(bb, 72);
    writeVarint32(bb, encodeStatus[$status]);
  }

  // optional Any output = 10;
  let $output = message.output;
  if ($output !== undefined) {
    writeVarint32(bb, 82);
    let nested = popByteBuffer();
    _encodeAny($output, nested);
    writeVarint32(bb, nested.limit);
    writeByteBuffer(bb, nested);
    pushByteBuffer(nested);
  }
}

export function decodeTask(binary) {
  return _decodeTask(wrapByteBuffer(binary));
}

function _decodeTask(bb) {
  let message = {};

  end_of_message: while (!isAtEnd(bb)) {
    let tag = readVarint32(bb);

    switch (tag >>> 3) {
      case 0:
        break end_of_message;

      // optional string name = 2;
      case 2: {
        message.name = readString(bb, readVarint32(bb));
        break;
      }

      // optional string receiver = 3;
      case 3: {
        message.receiver = readString(bb, readVarint32(bb));
        break;
      }

      // optional string endpoint = 4;
      case 4: {
        message.endpoint = readString(bb, readVarint32(bb));
        break;
      }

      // optional string access_token = 5;
      case 5: {
        message.access_token = readString(bb, readVarint32(bb));
        break;
      }

      // optional string job_id = 6;
      case 6: {
        message.job_id = readString(bb, readVarint32(bb));
        break;
      }

      // optional string sender = 7;
      case 7: {
        message.sender = readString(bb, readVarint32(bb));
        break;
      }

      // optional Status status = 9;
      case 9: {
        message.status = decodeStatus[readVarint32(bb)];
        break;
      }

      // optional Any output = 10;
      case 10: {
        let limit = pushTemporaryLength(bb);
        message.output = _decodeAny(bb);
        bb.limit = limit;
        break;
      }

      default:
        skipUnknownField(bb, tag & 7);
    }
  }

  return message;
}

export function encodeLocalRefinementInputV1(message) {
  let bb = popByteBuffer();
  _encodeLocalRefinementInputV1(message, bb);
  return toUint8Array(bb);
}

function _encodeLocalRefinementInputV1(message, bb) {
  // optional domain_data.Query query = 1;
  let $query = message.query;
  if ($query !== undefined) {
    writeVarint32(bb, 10);
    let nested = popByteBuffer();
    _encodedomain_data.Query($query, nested);
    writeVarint32(bb, nested.limit);
    writeByteBuffer(bb, nested);
    pushByteBuffer(nested);
  }
}

export function decodeLocalRefinementInputV1(binary) {
  return _decodeLocalRefinementInputV1(wrapByteBuffer(binary));
}

function _decodeLocalRefinementInputV1(bb) {
  let message = {};

  end_of_message: while (!isAtEnd(bb)) {
    let tag = readVarint32(bb);

    switch (tag >>> 3) {
      case 0:
        break end_of_message;

      // optional domain_data.Query query = 1;
      case 1: {
        let limit = pushTemporaryLength(bb);
        message.query = _decodedomain_data.Query(bb);
        bb.limit = limit;
        break;
      }

      default:
        skipUnknownField(bb, tag & 7);
    }
  }

  return message;
}

export function encodeDomainClusterHandshake(message) {
  let bb = popByteBuffer();
  _encodeDomainClusterHandshake(message, bb);
  return toUint8Array(bb);
}

function _encodeDomainClusterHandshake(message, bb) {
  // optional string access_token = 1;
  let $access_token = message.access_token;
  if ($access_token !== undefined) {
    writeVarint32(bb, 10);
    writeString(bb, $access_token);
  }
}

export function decodeDomainClusterHandshake(binary) {
  return _decodeDomainClusterHandshake(wrapByteBuffer(binary));
}

function _decodeDomainClusterHandshake(bb) {
  let message = {};

  end_of_message: while (!isAtEnd(bb)) {
    let tag = readVarint32(bb);

    switch (tag >>> 3) {
      case 0:
        break end_of_message;

      // optional string access_token = 1;
      case 1: {
        message.access_token = readString(bb, readVarint32(bb));
        break;
      }

      default:
        skipUnknownField(bb, tag & 7);
    }
  }

  return message;
}

export function encodeGlobalRefinementInputV1(message) {
  let bb = popByteBuffer();
  _encodeGlobalRefinementInputV1(message, bb);
  return toUint8Array(bb);
}

function _encodeGlobalRefinementInputV1(message, bb) {
  // repeated LocalRefinementOutputV1 local_refinement_results = 1;
  let array$local_refinement_results = message.local_refinement_results;
  if (array$local_refinement_results !== undefined) {
    for (let value of array$local_refinement_results) {
      writeVarint32(bb, 10);
      let nested = popByteBuffer();
      _encodeLocalRefinementOutputV1(value, nested);
      writeVarint32(bb, nested.limit);
      writeByteBuffer(bb, nested);
      pushByteBuffer(nested);
    }
  }
}

export function decodeGlobalRefinementInputV1(binary) {
  return _decodeGlobalRefinementInputV1(wrapByteBuffer(binary));
}

function _decodeGlobalRefinementInputV1(bb) {
  let message = {};

  end_of_message: while (!isAtEnd(bb)) {
    let tag = readVarint32(bb);

    switch (tag >>> 3) {
      case 0:
        break end_of_message;

      // repeated LocalRefinementOutputV1 local_refinement_results = 1;
      case 1: {
        let limit = pushTemporaryLength(bb);
        let values = message.local_refinement_results || (message.local_refinement_results = []);
        values.push(_decodeLocalRefinementOutputV1(bb));
        bb.limit = limit;
        break;
      }

      default:
        skipUnknownField(bb, tag & 7);
    }
  }

  return message;
}

export function encodeDomainDataChunk(message) {
  let bb = popByteBuffer();
  _encodeDomainDataChunk(message, bb);
  return toUint8Array(bb);
}

function _encodeDomainDataChunk(message, bb) {
  // optional bytes data = 1;
  let $data = message.data;
  if ($data !== undefined) {
    writeVarint32(bb, 10);
    writeVarint32(bb, $data.length), writeBytes(bb, $data);
  }
}

export function decodeDomainDataChunk(binary) {
  return _decodeDomainDataChunk(wrapByteBuffer(binary));
}

function _decodeDomainDataChunk(bb) {
  let message = {};

  end_of_message: while (!isAtEnd(bb)) {
    let tag = readVarint32(bb);

    switch (tag >>> 3) {
      case 0:
        break end_of_message;

      // optional bytes data = 1;
      case 1: {
        message.data = readBytes(bb, readVarint32(bb));
        break;
      }

      default:
        skipUnknownField(bb, tag & 7);
    }
  }

  return message;
}

function pushTemporaryLength(bb) {
  let length = readVarint32(bb);
  let limit = bb.limit;
  bb.limit = bb.offset + length;
  return limit;
}

function skipUnknownField(bb, type) {
  switch (type) {
    case 0: while (readByte(bb) & 0x80) { } break;
    case 2: skip(bb, readVarint32(bb)); break;
    case 5: skip(bb, 4); break;
    case 1: skip(bb, 8); break;
    default: throw new Error("Unimplemented type: " + type);
  }
}

function stringToLong(value) {
  return {
    low: value.charCodeAt(0) | (value.charCodeAt(1) << 16),
    high: value.charCodeAt(2) | (value.charCodeAt(3) << 16),
    unsigned: false,
  };
}

function longToString(value) {
  let low = value.low;
  let high = value.high;
  return String.fromCharCode(
    low & 0xFFFF,
    low >>> 16,
    high & 0xFFFF,
    high >>> 16);
}

// The code below was modified from https://github.com/protobufjs/bytebuffer.js
// which is under the Apache License 2.0.

let f32 = new Float32Array(1);
let f32_u8 = new Uint8Array(f32.buffer);

let f64 = new Float64Array(1);
let f64_u8 = new Uint8Array(f64.buffer);

function intToLong(value) {
  value |= 0;
  return {
    low: value,
    high: value >> 31,
    unsigned: value >= 0,
  };
}

let bbStack = [];

function popByteBuffer() {
  const bb = bbStack.pop();
  if (!bb) return { bytes: new Uint8Array(64), offset: 0, limit: 0 };
  bb.offset = bb.limit = 0;
  return bb;
}

function pushByteBuffer(bb) {
  bbStack.push(bb);
}

function wrapByteBuffer(bytes) {
  return { bytes, offset: 0, limit: bytes.length };
}

function toUint8Array(bb) {
  let bytes = bb.bytes;
  let limit = bb.limit;
  return bytes.length === limit ? bytes : bytes.subarray(0, limit);
}

function skip(bb, offset) {
  if (bb.offset + offset > bb.limit) {
    throw new Error('Skip past limit');
  }
  bb.offset += offset;
}

function isAtEnd(bb) {
  return bb.offset >= bb.limit;
}

function grow(bb, count) {
  let bytes = bb.bytes;
  let offset = bb.offset;
  let limit = bb.limit;
  let finalOffset = offset + count;
  if (finalOffset > bytes.length) {
    let newBytes = new Uint8Array(finalOffset * 2);
    newBytes.set(bytes);
    bb.bytes = newBytes;
  }
  bb.offset = finalOffset;
  if (finalOffset > limit) {
    bb.limit = finalOffset;
  }
  return offset;
}

function advance(bb, count) {
  let offset = bb.offset;
  if (offset + count > bb.limit) {
    throw new Error('Read past limit');
  }
  bb.offset += count;
  return offset;
}

function readBytes(bb, count) {
  let offset = advance(bb, count);
  return bb.bytes.subarray(offset, offset + count);
}

function writeBytes(bb, buffer) {
  let offset = grow(bb, buffer.length);
  bb.bytes.set(buffer, offset);
}

function readString(bb, count) {
  // Sadly a hand-coded UTF8 decoder is much faster than subarray+TextDecoder in V8
  let offset = advance(bb, count);
  let fromCharCode = String.fromCharCode;
  let bytes = bb.bytes;
  let invalid = '\uFFFD';
  let text = '';

  for (let i = 0; i < count; i++) {
    let c1 = bytes[i + offset], c2, c3, c4, c;

    // 1 byte
    if ((c1 & 0x80) === 0) {
      text += fromCharCode(c1);
    }

    // 2 bytes
    else if ((c1 & 0xE0) === 0xC0) {
      if (i + 1 >= count) text += invalid;
      else {
        c2 = bytes[i + offset + 1];
        if ((c2 & 0xC0) !== 0x80) text += invalid;
        else {
          c = ((c1 & 0x1F) << 6) | (c2 & 0x3F);
          if (c < 0x80) text += invalid;
          else {
            text += fromCharCode(c);
            i++;
          }
        }
      }
    }

    // 3 bytes
    else if ((c1 & 0xF0) == 0xE0) {
      if (i + 2 >= count) text += invalid;
      else {
        c2 = bytes[i + offset + 1];
        c3 = bytes[i + offset + 2];
        if (((c2 | (c3 << 8)) & 0xC0C0) !== 0x8080) text += invalid;
        else {
          c = ((c1 & 0x0F) << 12) | ((c2 & 0x3F) << 6) | (c3 & 0x3F);
          if (c < 0x0800 || (c >= 0xD800 && c <= 0xDFFF)) text += invalid;
          else {
            text += fromCharCode(c);
            i += 2;
          }
        }
      }
    }

    // 4 bytes
    else if ((c1 & 0xF8) == 0xF0) {
      if (i + 3 >= count) text += invalid;
      else {
        c2 = bytes[i + offset + 1];
        c3 = bytes[i + offset + 2];
        c4 = bytes[i + offset + 3];
        if (((c2 | (c3 << 8) | (c4 << 16)) & 0xC0C0C0) !== 0x808080) text += invalid;
        else {
          c = ((c1 & 0x07) << 0x12) | ((c2 & 0x3F) << 0x0C) | ((c3 & 0x3F) << 0x06) | (c4 & 0x3F);
          if (c < 0x10000 || c > 0x10FFFF) text += invalid;
          else {
            c -= 0x10000;
            text += fromCharCode((c >> 10) + 0xD800, (c & 0x3FF) + 0xDC00);
            i += 3;
          }
        }
      }
    }

    else text += invalid;
  }

  return text;
}

function writeString(bb, text) {
  // Sadly a hand-coded UTF8 encoder is much faster than TextEncoder+set in V8
  let n = text.length;
  let byteCount = 0;

  // Write the byte count first
  for (let i = 0; i < n; i++) {
    let c = text.charCodeAt(i);
    if (c >= 0xD800 && c <= 0xDBFF && i + 1 < n) {
      c = (c << 10) + text.charCodeAt(++i) - 0x35FDC00;
    }
    byteCount += c < 0x80 ? 1 : c < 0x800 ? 2 : c < 0x10000 ? 3 : 4;
  }
  writeVarint32(bb, byteCount);

  let offset = grow(bb, byteCount);
  let bytes = bb.bytes;

  // Then write the bytes
  for (let i = 0; i < n; i++) {
    let c = text.charCodeAt(i);
    if (c >= 0xD800 && c <= 0xDBFF && i + 1 < n) {
      c = (c << 10) + text.charCodeAt(++i) - 0x35FDC00;
    }
    if (c < 0x80) {
      bytes[offset++] = c;
    } else {
      if (c < 0x800) {
        bytes[offset++] = ((c >> 6) & 0x1F) | 0xC0;
      } else {
        if (c < 0x10000) {
          bytes[offset++] = ((c >> 12) & 0x0F) | 0xE0;
        } else {
          bytes[offset++] = ((c >> 18) & 0x07) | 0xF0;
          bytes[offset++] = ((c >> 12) & 0x3F) | 0x80;
        }
        bytes[offset++] = ((c >> 6) & 0x3F) | 0x80;
      }
      bytes[offset++] = (c & 0x3F) | 0x80;
    }
  }
}

function writeByteBuffer(bb, buffer) {
  let offset = grow(bb, buffer.limit);
  let from = bb.bytes;
  let to = buffer.bytes;

  // This for loop is much faster than subarray+set on V8
  for (let i = 0, n = buffer.limit; i < n; i++) {
    from[i + offset] = to[i];
  }
}

function readByte(bb) {
  return bb.bytes[advance(bb, 1)];
}

function writeByte(bb, value) {
  let offset = grow(bb, 1);
  bb.bytes[offset] = value;
}

function readFloat(bb) {
  let offset = advance(bb, 4);
  let bytes = bb.bytes;

  // Manual copying is much faster than subarray+set in V8
  f32_u8[0] = bytes[offset++];
  f32_u8[1] = bytes[offset++];
  f32_u8[2] = bytes[offset++];
  f32_u8[3] = bytes[offset++];
  return f32[0];
}

function writeFloat(bb, value) {
  let offset = grow(bb, 4);
  let bytes = bb.bytes;
  f32[0] = value;

  // Manual copying is much faster than subarray+set in V8
  bytes[offset++] = f32_u8[0];
  bytes[offset++] = f32_u8[1];
  bytes[offset++] = f32_u8[2];
  bytes[offset++] = f32_u8[3];
}

function readDouble(bb) {
  let offset = advance(bb, 8);
  let bytes = bb.bytes;

  // Manual copying is much faster than subarray+set in V8
  f64_u8[0] = bytes[offset++];
  f64_u8[1] = bytes[offset++];
  f64_u8[2] = bytes[offset++];
  f64_u8[3] = bytes[offset++];
  f64_u8[4] = bytes[offset++];
  f64_u8[5] = bytes[offset++];
  f64_u8[6] = bytes[offset++];
  f64_u8[7] = bytes[offset++];
  return f64[0];
}

function writeDouble(bb, value) {
  let offset = grow(bb, 8);
  let bytes = bb.bytes;
  f64[0] = value;

  // Manual copying is much faster than subarray+set in V8
  bytes[offset++] = f64_u8[0];
  bytes[offset++] = f64_u8[1];
  bytes[offset++] = f64_u8[2];
  bytes[offset++] = f64_u8[3];
  bytes[offset++] = f64_u8[4];
  bytes[offset++] = f64_u8[5];
  bytes[offset++] = f64_u8[6];
  bytes[offset++] = f64_u8[7];
}

function readInt32(bb) {
  let offset = advance(bb, 4);
  let bytes = bb.bytes;
  return (
    bytes[offset] |
    (bytes[offset + 1] << 8) |
    (bytes[offset + 2] << 16) |
    (bytes[offset + 3] << 24)
  );
}

function writeInt32(bb, value) {
  let offset = grow(bb, 4);
  let bytes = bb.bytes;
  bytes[offset] = value;
  bytes[offset + 1] = value >> 8;
  bytes[offset + 2] = value >> 16;
  bytes[offset + 3] = value >> 24;
}

function readInt64(bb, unsigned) {
  return {
    low: readInt32(bb),
    high: readInt32(bb),
    unsigned,
  };
}

function writeInt64(bb, value) {
  writeInt32(bb, value.low);
  writeInt32(bb, value.high);
}

function readVarint32(bb) {
  let c = 0;
  let value = 0;
  let b;
  do {
    b = readByte(bb);
    if (c < 32) value |= (b & 0x7F) << c;
    c += 7;
  } while (b & 0x80);
  return value;
}

function writeVarint32(bb, value) {
  value >>>= 0;
  while (value >= 0x80) {
    writeByte(bb, (value & 0x7f) | 0x80);
    value >>>= 7;
  }
  writeByte(bb, value);
}

function readVarint64(bb, unsigned) {
  let part0 = 0;
  let part1 = 0;
  let part2 = 0;
  let b;

  b = readByte(bb); part0 = (b & 0x7F); if (b & 0x80) {
    b = readByte(bb); part0 |= (b & 0x7F) << 7; if (b & 0x80) {
      b = readByte(bb); part0 |= (b & 0x7F) << 14; if (b & 0x80) {
        b = readByte(bb); part0 |= (b & 0x7F) << 21; if (b & 0x80) {

          b = readByte(bb); part1 = (b & 0x7F); if (b & 0x80) {
            b = readByte(bb); part1 |= (b & 0x7F) << 7; if (b & 0x80) {
              b = readByte(bb); part1 |= (b & 0x7F) << 14; if (b & 0x80) {
                b = readByte(bb); part1 |= (b & 0x7F) << 21; if (b & 0x80) {

                  b = readByte(bb); part2 = (b & 0x7F); if (b & 0x80) {
                    b = readByte(bb); part2 |= (b & 0x7F) << 7;
                  }
                }
              }
            }
          }
        }
      }
    }
  }

  return {
    low: part0 | (part1 << 28),
    high: (part1 >>> 4) | (part2 << 24),
    unsigned,
  };
}

function writeVarint64(bb, value) {
  let part0 = value.low >>> 0;
  let part1 = ((value.low >>> 28) | (value.high << 4)) >>> 0;
  let part2 = value.high >>> 24;

  // ref: src/google/protobuf/io/coded_stream.cc
  let size =
    part2 === 0 ?
      part1 === 0 ?
        part0 < 1 << 14 ?
          part0 < 1 << 7 ? 1 : 2 :
          part0 < 1 << 21 ? 3 : 4 :
        part1 < 1 << 14 ?
          part1 < 1 << 7 ? 5 : 6 :
          part1 < 1 << 21 ? 7 : 8 :
      part2 < 1 << 7 ? 9 : 10;

  let offset = grow(bb, size);
  let bytes = bb.bytes;

  switch (size) {
    case 10: bytes[offset + 9] = (part2 >>> 7) & 0x01;
    case 9: bytes[offset + 8] = size !== 9 ? part2 | 0x80 : part2 & 0x7F;
    case 8: bytes[offset + 7] = size !== 8 ? (part1 >>> 21) | 0x80 : (part1 >>> 21) & 0x7F;
    case 7: bytes[offset + 6] = size !== 7 ? (part1 >>> 14) | 0x80 : (part1 >>> 14) & 0x7F;
    case 6: bytes[offset + 5] = size !== 6 ? (part1 >>> 7) | 0x80 : (part1 >>> 7) & 0x7F;
    case 5: bytes[offset + 4] = size !== 5 ? part1 | 0x80 : part1 & 0x7F;
    case 4: bytes[offset + 3] = size !== 4 ? (part0 >>> 21) | 0x80 : (part0 >>> 21) & 0x7F;
    case 3: bytes[offset + 2] = size !== 3 ? (part0 >>> 14) | 0x80 : (part0 >>> 14) & 0x7F;
    case 2: bytes[offset + 1] = size !== 2 ? (part0 >>> 7) | 0x80 : (part0 >>> 7) & 0x7F;
    case 1: bytes[offset] = size !== 1 ? part0 | 0x80 : part0 & 0x7F;
  }
}

function readVarint32ZigZag(bb) {
  let value = readVarint32(bb);

  // ref: src/google/protobuf/wire_format_lite.h
  return (value >>> 1) ^ -(value & 1);
}

function writeVarint32ZigZag(bb, value) {
  // ref: src/google/protobuf/wire_format_lite.h
  writeVarint32(bb, (value << 1) ^ (value >> 31));
}

function readVarint64ZigZag(bb) {
  let value = readVarint64(bb, /* unsigned */ false);
  let low = value.low;
  let high = value.high;
  let flip = -(low & 1);

  // ref: src/google/protobuf/wire_format_lite.h
  return {
    low: ((low >>> 1) | (high << 31)) ^ flip,
    high: (high >>> 1) ^ flip,
    unsigned: false,
  };
}

function writeVarint64ZigZag(bb, value) {
  let low = value.low;
  let high = value.high;
  let flip = high >> 31;

  // ref: src/google/protobuf/wire_format_lite.h
  writeVarint64(bb, {
    low: (low << 1) ^ flip,
    high: ((high << 1) | (low >>> 31)) ^ flip,
    unsigned: false,
  });
}
