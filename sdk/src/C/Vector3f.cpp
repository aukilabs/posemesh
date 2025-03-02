/* This code is automatically generated from Vector3f.json interface. Do not modify it manually as it will be overwritten! */

#include <Posemesh/C/Vector3f.h>
#include <Posemesh/Vector3f.hpp>
#include <cassert>
#include <new>

psm_vector3f_t* psm_vector3f_create()
{
    return new (std::nothrow) psm::Vector3f;
}

psm_vector3f_t* psm_vector3f_duplicate(const psm_vector3f_t* vector3f)
{
    if (!vector3f) {
        assert(!"psm_vector3f_duplicate(): vector3f is null");
        return nullptr;
    }
    return new (std::nothrow) psm::Vector3f(*vector3f);
}

void psm_vector3f_destroy(psm_vector3f_t* vector3f)
{
    delete vector3f;
}

uint8_t psm_vector3f_equals(const psm_vector3f_t* vector3f, const psm_vector3f_t* other_vector3f)
{
    if (!vector3f) {
        assert(!"psm_vector3f_equals(): vector3f is null");
        return 0;
    }
    if (!other_vector3f) {
        assert(!"psm_vector3f_equals(): other_vector3f is null");
        return 0;
    }
    return static_cast<uint8_t>(vector3f->operator==(*other_vector3f));
}

size_t psm_vector3f_hash(const psm_vector3f_t* vector3f)
{
    if (!vector3f) {
        assert(!"psm_vector3f_hash(): vector3f is null");
        return 0;
    }
    return std::hash<psm::Vector3f> {}(*vector3f);
}

float psm_vector3f_get_x(const psm_vector3f_t* vector3f)
{
    if (!vector3f) {
        assert(!"psm_vector3f_get_x(): vector3f is null");
        return 0.0f;
    }
    return vector3f->getX();
}

void psm_vector3f_set_x(psm_vector3f_t* vector3f, float x)
{
    if (!vector3f) {
        assert(!"psm_vector3f_set_x(): vector3f is null");
        return;
    }
    vector3f->setX(x);
}

float psm_vector3f_get_y(const psm_vector3f_t* vector3f)
{
    if (!vector3f) {
        assert(!"psm_vector3f_get_y(): vector3f is null");
        return 0.0f;
    }
    return vector3f->getY();
}

void psm_vector3f_set_y(psm_vector3f_t* vector3f, float y)
{
    if (!vector3f) {
        assert(!"psm_vector3f_set_y(): vector3f is null");
        return;
    }
    vector3f->setY(y);
}

float psm_vector3f_get_z(const psm_vector3f_t* vector3f)
{
    if (!vector3f) {
        assert(!"psm_vector3f_get_z(): vector3f is null");
        return 0.0f;
    }
    return vector3f->getZ();
}

void psm_vector3f_set_z(psm_vector3f_t* vector3f, float z)
{
    if (!vector3f) {
        assert(!"psm_vector3f_set_z(): vector3f is null");
        return;
    }
    vector3f->setZ(z);
}

psm_vector3f_ref_t* psm_vector3f_ref_make(psm_vector3f_t* vector3f)
{
    return new (std::nothrow) psm_vector3f_ref_t(vector3f, &psm_vector3f_destroy);
}

psm_vector3f_ref_t* psm_vector3f_ref_clone(const psm_vector3f_ref_t* vector3f_ref)
{
    if (!vector3f_ref) {
        return nullptr;
    }
    return new (std::nothrow) psm_vector3f_ref_t(*vector3f_ref);
}

psm_vector3f_t* psm_vector3f_ref_get(const psm_vector3f_ref_t* vector3f_ref)
{
    if (!vector3f_ref) {
        return nullptr;
    }
    return vector3f_ref->get();
}

void psm_vector3f_ref_delete(psm_vector3f_ref_t* vector3f_ref)
{
    delete vector3f_ref;
}
