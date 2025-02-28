/* This code is automatically generated from Vector2f.json interface. Do not modify it manually as it will be overwritten! */

#include <Posemesh/C/Vector2f.h>
#include <Posemesh/Vector2f.hpp>
#include <cassert>
#include <new>

psm_vector2f_t* psm_vector2f_create()
{
    return new (std::nothrow) psm::Vector2f;
}

psm_vector2f_t* psm_vector2f_duplicate(const psm_vector2f_t* vector2f)
{
    if (!vector2f) {
        assert(!"psm_vector2f_duplicate(): vector2f is null");
        return nullptr;
    }
    return new (std::nothrow) psm::Vector2f(*vector2f);
}

void psm_vector2f_destroy(psm_vector2f_t* vector2f)
{
    delete vector2f;
}

uint8_t psm_vector2f_equals(const psm_vector2f_t* vector2f, const psm_vector2f_t* other_vector2f)
{
    if (!vector2f) {
        assert(!"psm_vector2f_equals(): vector2f is null");
        return 0;
    }
    if (!other_vector2f) {
        assert(!"psm_vector2f_equals(): other_vector2f is null");
        return 0;
    }
    return static_cast<uint8_t>(vector2f->operator==(*other_vector2f));
}

size_t psm_vector2f_hash(const psm_vector2f_t* vector2f)
{
    if (!vector2f) {
        assert(!"psm_vector2f_hash(): vector2f is null");
        return 0;
    }
    return std::hash<psm::Vector2f> {}(*vector2f);
}

float psm_vector2f_get_x(const psm_vector2f_t* vector2f)
{
    if (!vector2f) {
        assert(!"psm_vector2f_get_x(): vector2f is null");
        return 0.0f;
    }
    return vector2f->getX();
}

void psm_vector2f_set_x(psm_vector2f_t* vector2f, float x)
{
    if (!vector2f) {
        assert(!"psm_vector2f_set_x(): vector2f is null");
        return;
    }
    vector2f->setX(x);
}

float psm_vector2f_get_y(const psm_vector2f_t* vector2f)
{
    if (!vector2f) {
        assert(!"psm_vector2f_get_y(): vector2f is null");
        return 0.0f;
    }
    return vector2f->getY();
}

void psm_vector2f_set_y(psm_vector2f_t* vector2f, float y)
{
    if (!vector2f) {
        assert(!"psm_vector2f_set_y(): vector2f is null");
        return;
    }
    vector2f->setY(y);
}

psm_vector2f_ref_t* psm_vector2f_ref_make(psm_vector2f_t* vector2f)
{
    return new (std::nothrow) psm_vector2f_ref_t(vector2f, &psm_vector2f_destroy);
}

psm_vector2f_ref_t* psm_vector2f_ref_clone(const psm_vector2f_ref_t* vector2f_ref)
{
    if (!vector2f_ref) {
        return nullptr;
    }
    return new (std::nothrow) psm_vector2f_ref_t(*vector2f_ref);
}

psm_vector2f_t* psm_vector2f_ref_get(const psm_vector2f_ref_t* vector2f_ref)
{
    if (!vector2f_ref) {
        return nullptr;
    }
    return vector2f_ref->get();
}

void psm_vector2f_ref_delete(psm_vector2f_ref_t* vector2f_ref)
{
    delete vector2f_ref;
}
