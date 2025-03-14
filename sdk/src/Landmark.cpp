#include <Posemesh/Landmark.hpp>
#include <utility>

namespace psm {

Landmark::Landmark() = default;

Landmark::Landmark(const Landmark& landmark) = default;

Landmark::Landmark(Landmark&& landmark) = default;

Landmark::~Landmark() = default;

Landmark& Landmark::operator=(const Landmark& landmark) = default;

Landmark& Landmark::operator=(Landmark&& landmark) = default;

bool Landmark::operator==(const Landmark& landmark) const noexcept
{
    if (!(m_type == landmark.m_type)) {
        return false;
    }
    if (!(m_id == landmark.m_id)) {
        return false;
    }
    if (!(m_position == landmark.m_position)) {
        return false;
    }
    return true;
}

bool Landmark::operator!=(const Landmark& landmark) const noexcept
{
    return !(*this == landmark);
}

std::string Landmark::getType() const
{
    return m_type;
}

void Landmark::setType(std::string type) noexcept
{
    m_type = std::move(type);
}

std::string Landmark::getId() const
{
    return m_id;
}

void Landmark::setId(std::string id) noexcept
{
    m_id = std::move(id);
}

Vector3 Landmark::getPosition() const
{
    return m_position;
}

void Landmark::setPosition(Vector3 position) noexcept
{
    m_position = std::move(position);
}

}

namespace std {

std::size_t hash<psm::Landmark>::operator()(const psm::Landmark& landmark) const noexcept
{
    std::size_t result = 0;
    result ^= (hash<string> {}(landmark.m_type)) + 0x9e3779b9 + (result << 6) + (result >> 2);
    result ^= (hash<string> {}(landmark.m_id)) + 0x9e3779b9 + (result << 6) + (result >> 2);
    result ^= (hash<psm::Vector3> {}(landmark.m_position)) + 0x9e3779b9 + (result << 6) + (result >> 2);
    return result;
}

}
