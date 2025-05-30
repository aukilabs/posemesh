#ifndef __PORTALS_HPP__
#define __PORTALS_HPP__

#include <algorithm>
#include <string>

namespace psm {

class Portals final {
public:
    static bool isAukiQR(const std::string& stringToCheck)
    {
        // Case-insensitive string search.
        auto it = std::search(
            stringToCheck.begin(),
            stringToCheck.end(),
            m_aukiLighthousePrefix.begin(),
            m_aukiLighthousePrefix.end(),
            [](unsigned char ch1, unsigned char ch2) {
                return std::toupper(ch1) == std::toupper(ch2);
            });
        return (it != stringToCheck.end());
    }

    static std::string extractShortId(const std::string& portalContents)
    {
        return portalContents.substr(m_aukiLighthousePrefix.size());
    }

private:
    Portals() = delete;
    inline static std::string m_aukiLighthousePrefix = "HTTPS://R8.HR/";
};

}

#endif // __PORTALS_HPP__
