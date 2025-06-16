#include <Posemesh/CompileTestMethods.hpp>

namespace psm {

void CompileTestMethods::voidMethod() { }

std::int8_t CompileTestMethods::int8Method(std::int8_t firstParameter, std::int8_t secondParameter)
{
    return 0;
}

std::int16_t CompileTestMethods::int16Method(std::int16_t firstParameter, std::int16_t secondParameter)
{
    return 0;
}

std::int32_t CompileTestMethods::int32Method(std::int32_t firstParameter, std::int32_t secondParameter)
{
    return 0;
}

std::int64_t CompileTestMethods::int64Method(std::int64_t firstParameter, std::int64_t secondParameter)
{
    return 0;
}

std::uint8_t CompileTestMethods::unsignedInt8Method(std::uint8_t firstParameter, std::uint8_t secondParameter)
{
    return 0;
}

std::uint16_t CompileTestMethods::unsignedInt16Method(std::uint16_t firstParameter, std::uint16_t secondParameter)
{
    return 0;
}

std::uint32_t CompileTestMethods::unsignedInt32Method(std::uint32_t firstParameter, std::uint32_t secondParameter)
{
    return 0;
}

std::uint64_t CompileTestMethods::unsignedInt64Method(std::uint64_t firstParameter, std::uint64_t secondParameter)
{
    return 0;
}

float CompileTestMethods::floatMethod(float firstParameter, float secondParameter)
{
    return 0;
}

double CompileTestMethods::doubleMethod(double firstParameter, double secondParameter)
{
    return 0;
}

bool CompileTestMethods::boolMethod(bool firstParameter, bool secondParameter)
{
    return false;
}

std::string CompileTestMethods::stringMethod(std::string firstParameter, std::string secondParameter)
{
    return {};
}

const std::string& CompileTestMethods::stringRefMethod(const std::string& firstParameter, const std::string& secondParameter)
{
    static std::string value;
    return value;
}

CompileTestExampleEnum CompileTestMethods::enumMethod(CompileTestExampleEnum firstParameter, CompileTestExampleEnum secondParameter)
{
    return CompileTestExampleEnum::FirstConstant;
}

CompileTestDummyClass CompileTestMethods::classMethod(CompileTestDummyClass firstParameter, CompileTestDummyClass secondParameter)
{
    return {};
}

const CompileTestDummyClass& CompileTestMethods::classRefMethod(const CompileTestDummyClass& firstParameter, const CompileTestDummyClass& secondParameter)
{
    static CompileTestDummyClass value;
    return value;
}

std::shared_ptr<CompileTestDummyClass> CompileTestMethods::classPtrMethod(std::shared_ptr<CompileTestDummyClass> firstParameter, std::shared_ptr<CompileTestDummyClass> secondParameter)
{
    return {};
}

const std::shared_ptr<CompileTestDummyClass>& CompileTestMethods::classPtrRefMethod(const std::shared_ptr<CompileTestDummyClass>& firstParameter, const std::shared_ptr<CompileTestDummyClass>& secondParameter)
{
    static std::shared_ptr<CompileTestDummyClass> value;
    return value;
}

std::vector<std::int8_t> CompileTestMethods::arrayInt8Method(std::vector<std::int8_t> firstParameter, std::vector<std::int8_t> secondParameter)
{
    return {};
}

std::vector<std::int16_t> CompileTestMethods::arrayInt16Method(std::vector<std::int16_t> firstParameter, std::vector<std::int16_t> secondParameter)
{
    return {};
}

std::vector<std::int32_t> CompileTestMethods::arrayInt32Method(std::vector<std::int32_t> firstParameter, std::vector<std::int32_t> secondParameter)
{
    return {};
}

std::vector<std::int64_t> CompileTestMethods::arrayInt64Method(std::vector<std::int64_t> firstParameter, std::vector<std::int64_t> secondParameter)
{
    return {};
}

std::vector<std::uint8_t> CompileTestMethods::arrayUnsignedInt8Method(std::vector<std::uint8_t> firstParameter, std::vector<std::uint8_t> secondParameter)
{
    return {};
}

std::vector<std::uint16_t> CompileTestMethods::arrayUnsignedInt16Method(std::vector<std::uint16_t> firstParameter, std::vector<std::uint16_t> secondParameter)
{
    return {};
}

std::vector<std::uint32_t> CompileTestMethods::arrayUnsignedInt32Method(std::vector<std::uint32_t> firstParameter, std::vector<std::uint32_t> secondParameter)
{
    return {};
}

std::vector<std::uint64_t> CompileTestMethods::arrayUnsignedInt64Method(std::vector<std::uint64_t> firstParameter, std::vector<std::uint64_t> secondParameter)
{
    return {};
}

std::vector<float> CompileTestMethods::arrayFloatMethod(std::vector<float> firstParameter, std::vector<float> secondParameter)
{
    return {};
}

std::vector<double> CompileTestMethods::arrayDoubleMethod(std::vector<double> firstParameter, std::vector<double> secondParameter)
{
    return {};
}

std::vector<bool> CompileTestMethods::arrayBoolMethod(std::vector<bool> firstParameter, std::vector<bool> secondParameter)
{
    return {};
}

std::vector<std::string> CompileTestMethods::arrayStringMethod(std::vector<std::string> firstParameter, std::vector<std::string> secondParameter)
{
    return {};
}

std::vector<CompileTestExampleEnum> CompileTestMethods::arrayEnumMethod(std::vector<CompileTestExampleEnum> firstParameter, std::vector<CompileTestExampleEnum> secondParameter)
{
    return {};
}

std::vector<CompileTestDummyClass> CompileTestMethods::arrayClassMethod(std::vector<CompileTestDummyClass> firstParameter, std::vector<CompileTestDummyClass> secondParameter)
{
    return {};
}

const std::vector<std::int8_t>& CompileTestMethods::arrayRefInt8Method(const std::vector<std::int8_t>& firstParameter, const std::vector<std::int8_t>& secondParameter)
{
    static std::vector<std::int8_t> value;
    return value;
}

const std::vector<std::int16_t>& CompileTestMethods::arrayRefInt16Method(const std::vector<std::int16_t>& firstParameter, const std::vector<std::int16_t>& secondParameter)
{
    static std::vector<std::int16_t> value;
    return value;
}

const std::vector<std::int32_t>& CompileTestMethods::arrayRefInt32Method(const std::vector<std::int32_t>& firstParameter, const std::vector<std::int32_t>& secondParameter)
{
    static std::vector<std::int32_t> value;
    return value;
}

const std::vector<std::int64_t>& CompileTestMethods::arrayRefInt64Method(const std::vector<std::int64_t>& firstParameter, const std::vector<std::int64_t>& secondParameter)
{
    static std::vector<std::int64_t> value;
    return value;
}

const std::vector<std::uint8_t>& CompileTestMethods::arrayRefUnsignedInt8Method(const std::vector<std::uint8_t>& firstParameter, const std::vector<std::uint8_t>& secondParameter)
{
    static std::vector<std::uint8_t> value;
    return value;
}

const std::vector<std::uint16_t>& CompileTestMethods::arrayRefUnsignedInt16Method(const std::vector<std::uint16_t>& firstParameter, const std::vector<std::uint16_t>& secondParameter)
{
    static std::vector<std::uint16_t> value;
    return value;
}

const std::vector<std::uint32_t>& CompileTestMethods::arrayRefUnsignedInt32Method(const std::vector<std::uint32_t>& firstParameter, const std::vector<std::uint32_t>& secondParameter)
{
    static std::vector<std::uint32_t> value;
    return value;
}

const std::vector<std::uint64_t>& CompileTestMethods::arrayRefUnsignedInt64Method(const std::vector<std::uint64_t>& firstParameter, const std::vector<std::uint64_t>& secondParameter)
{
    static std::vector<std::uint64_t> value;
    return value;
}

const std::vector<float>& CompileTestMethods::arrayRefFloatMethod(const std::vector<float>& firstParameter, const std::vector<float>& secondParameter)
{
    static std::vector<float> value;
    return value;
}

const std::vector<double>& CompileTestMethods::arrayRefDoubleMethod(const std::vector<double>& firstParameter, const std::vector<double>& secondParameter)
{
    static std::vector<double> value;
    return value;
}

const std::vector<bool>& CompileTestMethods::arrayRefBoolMethod(const std::vector<bool>& firstParameter, const std::vector<bool>& secondParameter)
{
    static std::vector<bool> value;
    return value;
}

const std::vector<std::string>& CompileTestMethods::arrayRefStringMethod(const std::vector<std::string>& firstParameter, const std::vector<std::string>& secondParameter)
{
    static std::vector<std::string> value;
    return value;
}

const std::vector<CompileTestExampleEnum>& CompileTestMethods::arrayRefEnumMethod(const std::vector<CompileTestExampleEnum>& firstParameter, const std::vector<CompileTestExampleEnum>& secondParameter)
{
    static std::vector<CompileTestExampleEnum> value;
    return value;
}

const std::vector<CompileTestDummyClass>& CompileTestMethods::arrayRefClassMethod(const std::vector<CompileTestDummyClass>& firstParameter, const std::vector<CompileTestDummyClass>& secondParameter)
{
    static std::vector<CompileTestDummyClass> value;
    return value;
}

std::vector<std::shared_ptr<CompileTestDummyClass>> CompileTestMethods::arrayPtrClassMethod(std::vector<std::shared_ptr<CompileTestDummyClass>> firstParameter, std::vector<std::shared_ptr<CompileTestDummyClass>> secondParameter)
{
    return {};
}

const std::vector<std::shared_ptr<CompileTestDummyClass>>& CompileTestMethods::arrayPtrRefClassMethod(const std::vector<std::shared_ptr<CompileTestDummyClass>>& firstParameter, const std::vector<std::shared_ptr<CompileTestDummyClass>>& secondParameter)
{
    static std::vector<std::shared_ptr<CompileTestDummyClass>> value;
    return value;
}

const std::uint8_t* CompileTestMethods::dataMethod(const std::uint8_t* firstParameter, std::size_t firstParameterSize, const std::uint8_t* secondParameter, std::size_t secondParameterSize, std::size_t& outSize)
{
    outSize = 0;
    return nullptr;
}

}
