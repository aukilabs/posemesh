#include <Posemesh/C/Landmark.h>
#include <Posemesh/Landmark.hpp>
#include <cassert>
#include <cstring>
#include <new>

psm_landmark_t* psm_landmark_create()
{
    return new (std::nothrow) psm::Landmark;
}

psm_landmark_t* psm_landmark_duplicate(const psm_landmark_t* landmark)
{
    if (!landmark) {
        assert(!"psm_landmark_duplicate(): landmark is null");
        return nullptr;
    }
    return new (std::nothrow) psm::Landmark(*landmark);
}

void psm_landmark_destroy(psm_landmark_t* landmark)
{
    delete landmark;
}

uint8_t psm_landmark_equals(const psm_landmark_t* landmark, const psm_landmark_t* other_landmark)
{
    if (!landmark) {
        assert(!"psm_landmark_equals(): landmark is null");
        return 0;
    }
    if (!other_landmark) {
        assert(!"psm_landmark_equals(): other_landmark is null");
        return 0;
    }
    return static_cast<uint8_t>(landmark->operator==(*other_landmark));
}

size_t psm_landmark_hash(const psm_landmark_t* landmark)
{
    if (!landmark) {
        assert(!"psm_landmark_hash(): landmark is null");
        return 0;
    }
    return std::hash<psm::Landmark> {}(*landmark);
}

const char* psm_landmark_get_type(const psm_landmark_t* landmark)
{
    if (!landmark) {
        assert(!"psm_landmark_get_type(): landmark is null");
        return nullptr;
    }
    const auto type = landmark->getType();
    auto* getter_result = new (std::nothrow) char[type.size() + 1];
    if (!getter_result) {
        return nullptr;
    }
    std::memcpy(getter_result, type.c_str(), type.size() + 1);
    return getter_result;
}

void psm_landmark_get_type_free(const char* type)
{
    delete[] const_cast<char*>(type);
}

void psm_landmark_set_type(psm_landmark_t* landmark, const char* type)
{
    if (!landmark) {
        assert(!"psm_landmark_set_type(): landmark is null");
        return;
    }
    landmark->setType(type ? std::string { type } : std::string {});
}

const char* psm_landmark_get_id(const psm_landmark_t* landmark)
{
    if (!landmark) {
        assert(!"psm_landmark_get_id(): landmark is null");
        return nullptr;
    }
    const auto id = landmark->getId();
    auto* getter_result = new (std::nothrow) char[id.size() + 1];
    if (!getter_result) {
        return nullptr;
    }
    std::memcpy(getter_result, id.c_str(), id.size() + 1);
    return getter_result;
}

void psm_landmark_get_id_free(const char* id)
{
    delete[] const_cast<char*>(id);
}

void psm_landmark_set_id(psm_landmark_t* landmark, const char* id)
{
    if (!landmark) {
        assert(!"psm_landmark_set_id(): landmark is null");
        return;
    }
    landmark->setId(id ? std::string { id } : std::string {});
}

psm_vector3_t* psm_landmark_get_position(const psm_landmark_t* landmark)
{
    if (!landmark) {
        assert(!"psm_landmark_get_position(): landmark is null");
        return nullptr;
    }
    const auto position = landmark->getPosition();
    psm_vector3_t* landmark_position = psm_vector3_create();
    psm_vector3_set_x(landmark_position, position.getX());
    psm_vector3_set_y(landmark_position, position.getY());
    psm_vector3_set_z(landmark_position, position.getZ());
    return landmark_position;
}

void psm_landmark_get_position_free(psm_vector3_t* position)
{
    psm_vector3_destroy(position);
}

void psm_landmark_set_position(psm_landmark_t* landmark, psm_vector3_t* position)
{
    if (!landmark) {
        assert(!"psm_landmark_set_position(): landmark is null");
        return;
    }
    if (position) {
        psm::Vector3 v;
        v.setX(psm_vector3_get_x(position));
        v.setY(psm_vector3_get_y(position));
        v.setZ(psm_vector3_get_z(position));
        landmark->setPosition(v);
    } else {
        landmark->setPosition(psm::Vector3());
    }
}

psm_landmark_ref_t* psm_landmark_ref_make(psm_landmark_t* landmark)
{
    return new (std::nothrow) psm_landmark_ref_t(landmark, &psm_landmark_destroy);
}

psm_landmark_ref_t* psm_landmark_ref_clone(const psm_landmark_ref_t* landmark_ref)
{
    if (!landmark_ref) {
        return nullptr;
    }
    return new (std::nothrow) psm_landmark_ref_t(*landmark_ref);
}

psm_landmark_t* psm_landmark_ref_get(const psm_landmark_ref_t* landmark_ref)
{
    if (!landmark_ref) {
        return nullptr;
    }
    return landmark_ref->get();
}

void psm_landmark_ref_delete(psm_landmark_ref_t* landmark_ref)
{
    delete landmark_ref;
}
