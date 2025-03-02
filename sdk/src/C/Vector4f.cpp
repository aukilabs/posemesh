/* This code is automatically generated from Vector4f.json interface. Do not modify it manually as it will be overwritten! */

#include <Posemesh/C/Vector4f.h>
#include <Posemesh/Vector4f.hpp>
#include <cassert>
#include <new>

psm_vector4f_t* psm_vector4f_create()
{
    return new (std::nothrow) psm::Vector4f;
}

psm_vector4f_t* psm_vector4f_duplicate(const psm_vector4f_t* vector4f)
{
    if (!vector4f) {
        assert(!"psm_vector4f_duplicate(): vector4f is null");
        return nullptr;
    }
    return new (std::nothrow) psm::Vector4f(*vector4f);
}

void psm_vector4f_destroy(psm_vector4f_t* vector4f)
{
    delete vector4f;
}

uint8_t psm_vector4f_equals(const psm_vector4f_t* vector4f, const psm_vector4f_t* other_vector4f)
{
    if (!vector4f) {
        assert(!"psm_vector4f_equals(): vector4f is null");
        return 0;
    }
    if (!other_vector4f) {
        assert(!"psm_vector4f_equals(): other_vector4f is null");
        return 0;
    }
    return static_cast<uint8_t>(vector4f->operator==(*other_vector4f));
}

size_t psm_vector4f_hash(const psm_vector4f_t* vector4f)
{
    if (!vector4f) {
        assert(!"psm_vector4f_hash(): vector4f is null");
        return 0;
    }
    return std::hash<psm::Vector4f> {}(*vector4f);
}

float psm_vector4f_get_x(const psm_vector4f_t* vector4f)
{
    if (!vector4f) {
        assert(!"psm_vector4f_get_x(): vector4f is null");
        return 0.0f;
    }
    return vector4f->getX();
}

void psm_vector4f_set_x(psm_vector4f_t* vector4f, float x)
{
    if (!vector4f) {
        assert(!"psm_vector4f_set_x(): vector4f is null");
        return;
    }
    vector4f->setX(x);
}

float psm_vector4f_get_y(const psm_vector4f_t* vector4f)
{
    if (!vector4f) {
        assert(!"psm_vector4f_get_y(): vector4f is null");
        return 0.0f;
    }
    return vector4f->getY();
}

void psm_vector4f_set_y(psm_vector4f_t* vector4f, float y)
{
    if (!vector4f) {
        assert(!"psm_vector4f_set_y(): vector4f is null");
        return;
    }
    vector4f->setY(y);
}

float psm_vector4f_get_z(const psm_vector4f_t* vector4f)
{
    if (!vector4f) {
        assert(!"psm_vector4f_get_z(): vector4f is null");
        return 0.0f;
    }
    return vector4f->getZ();
}

void psm_vector4f_set_z(psm_vector4f_t* vector4f, float z)
{
    if (!vector4f) {
        assert(!"psm_vector4f_set_z(): vector4f is null");
        return;
    }
    vector4f->setZ(z);
}

float psm_vector4f_get_w(const psm_vector4f_t* vector4f)
{
    if (!vector4f) {
        assert(!"psm_vector4f_get_w(): vector4f is null");
        return 0.0f;
    }
    return vector4f->getW();
}

void psm_vector4f_set_w(psm_vector4f_t* vector4f, float w)
{
    if (!vector4f) {
        assert(!"psm_vector4f_set_w(): vector4f is null");
        return;
    }
    vector4f->setW(w);
}

psm_vector4f_ref_t* psm_vector4f_ref_make(psm_vector4f_t* vector4f)
{
    return new (std::nothrow) psm_vector4f_ref_t(vector4f, &psm_vector4f_destroy);
}

psm_vector4f_ref_t* psm_vector4f_ref_clone(const psm_vector4f_ref_t* vector4f_ref)
{
    if (!vector4f_ref) {
        return nullptr;
    }
    return new (std::nothrow) psm_vector4f_ref_t(*vector4f_ref);
}

psm_vector4f_t* psm_vector4f_ref_get(const psm_vector4f_ref_t* vector4f_ref)
{
    if (!vector4f_ref) {
        return nullptr;
    }
    return vector4f_ref->get();
}

void psm_vector4f_ref_delete(psm_vector4f_ref_t* vector4f_ref)
{
    delete vector4f_ref;
}
