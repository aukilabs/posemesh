#include <Posemesh/CompileTestFunctions.hpp>

namespace psm {

std::vector<std::uint16_t> CompileTestFunctions::complexMethod(const std::string& firstParameter, CompileTestExampleEnum secondParameter, const std::vector<CompileTestExampleEnum>& thirdParameter)
{
    return {};
}

void CompileTestFunctions::voidMethod() { }

std::int8_t CompileTestFunctions::int8Method(std::int8_t firstParameter, std::int8_t secondParameter)
{
    return 0;
}

std::int16_t CompileTestFunctions::int16Method(std::int16_t firstParameter, std::int16_t secondParameter)
{
    return 0;
}

std::int32_t CompileTestFunctions::int32Method(std::int32_t firstParameter, std::int32_t secondParameter)
{
    return 0;
}

std::int64_t CompileTestFunctions::int64Method(std::int64_t firstParameter, std::int64_t secondParameter)
{
    return 0;
}

std::uint8_t CompileTestFunctions::unsignedInt8Method(std::uint8_t firstParameter, std::uint8_t secondParameter)
{
    return 0;
}

std::uint16_t CompileTestFunctions::unsignedInt16Method(std::uint16_t firstParameter, std::uint16_t secondParameter)
{
    return 0;
}

std::uint32_t CompileTestFunctions::unsignedInt32Method(std::uint32_t firstParameter, std::uint32_t secondParameter)
{
    return 0;
}

std::uint64_t CompileTestFunctions::unsignedInt64Method(std::uint64_t firstParameter, std::uint64_t secondParameter)
{
    return 0;
}

float CompileTestFunctions::floatMethod(float firstParameter, float secondParameter)
{
    return 0;
}

double CompileTestFunctions::doubleMethod(double firstParameter, double secondParameter)
{
    return 0;
}

bool CompileTestFunctions::boolMethod(bool firstParameter, bool secondParameter)
{
    return false;
}

std::string CompileTestFunctions::stringMethod(std::string firstParameter, std::string secondParameter)
{
    return {};
}

const std::string& CompileTestFunctions::stringRefMethod(const std::string& firstParameter, const std::string& secondParameter)
{
    static std::string value;
    return value;
}

CompileTestExampleEnum CompileTestFunctions::enumMethod(CompileTestExampleEnum firstParameter, CompileTestExampleEnum secondParameter)
{
    return CompileTestExampleEnum::FirstConstant;
}

CompileTestDummyClass CompileTestFunctions::classMethod(CompileTestDummyClass firstParameter, CompileTestDummyClass secondParameter)
{
    return {};
}

const CompileTestDummyClass& CompileTestFunctions::classRefMethod(const CompileTestDummyClass& firstParameter, const CompileTestDummyClass& secondParameter)
{
    static CompileTestDummyClass value;
    return value;
}

std::shared_ptr<CompileTestDummyClass> CompileTestFunctions::classPtrMethod(std::shared_ptr<CompileTestDummyClass> firstParameter, std::shared_ptr<CompileTestDummyClass> secondParameter)
{
    return {};
}

const std::shared_ptr<CompileTestDummyClass>& CompileTestFunctions::classPtrRefMethod(const std::shared_ptr<CompileTestDummyClass>& firstParameter, const std::shared_ptr<CompileTestDummyClass>& secondParameter)
{
    static std::shared_ptr<CompileTestDummyClass> value;
    return value;
}

std::vector<std::int8_t> CompileTestFunctions::arrayInt8Method(std::vector<std::int8_t> firstParameter, std::vector<std::int8_t> secondParameter)
{
    return {};
}

std::vector<std::int16_t> CompileTestFunctions::arrayInt16Method(std::vector<std::int16_t> firstParameter, std::vector<std::int16_t> secondParameter)
{
    return {};
}

std::vector<std::int32_t> CompileTestFunctions::arrayInt32Method(std::vector<std::int32_t> firstParameter, std::vector<std::int32_t> secondParameter)
{
    return {};
}

std::vector<std::int64_t> CompileTestFunctions::arrayInt64Method(std::vector<std::int64_t> firstParameter, std::vector<std::int64_t> secondParameter)
{
    return {};
}

std::vector<std::uint8_t> CompileTestFunctions::arrayUnsignedInt8Method(std::vector<std::uint8_t> firstParameter, std::vector<std::uint8_t> secondParameter)
{
    return {};
}

std::vector<std::uint16_t> CompileTestFunctions::arrayUnsignedInt16Method(std::vector<std::uint16_t> firstParameter, std::vector<std::uint16_t> secondParameter)
{
    return {};
}

std::vector<std::uint32_t> CompileTestFunctions::arrayUnsignedInt32Method(std::vector<std::uint32_t> firstParameter, std::vector<std::uint32_t> secondParameter)
{
    return {};
}

std::vector<std::uint64_t> CompileTestFunctions::arrayUnsignedInt64Method(std::vector<std::uint64_t> firstParameter, std::vector<std::uint64_t> secondParameter)
{
    return {};
}

std::vector<float> CompileTestFunctions::arrayFloatMethod(std::vector<float> firstParameter, std::vector<float> secondParameter)
{
    return {};
}

std::vector<double> CompileTestFunctions::arrayDoubleMethod(std::vector<double> firstParameter, std::vector<double> secondParameter)
{
    return {};
}

std::vector<bool> CompileTestFunctions::arrayBoolMethod(std::vector<bool> firstParameter, std::vector<bool> secondParameter)
{
    return {};
}

std::vector<std::string> CompileTestFunctions::arrayStringMethod(std::vector<std::string> firstParameter, std::vector<std::string> secondParameter)
{
    return {};
}

std::vector<CompileTestExampleEnum> CompileTestFunctions::arrayEnumMethod(std::vector<CompileTestExampleEnum> firstParameter, std::vector<CompileTestExampleEnum> secondParameter)
{
    return {};
}

std::vector<CompileTestDummyClass> CompileTestFunctions::arrayClassMethod(std::vector<CompileTestDummyClass> firstParameter, std::vector<CompileTestDummyClass> secondParameter)
{
    return {};
}

const std::vector<std::int8_t>& CompileTestFunctions::arrayRefInt8Method(const std::vector<std::int8_t>& firstParameter, const std::vector<std::int8_t>& secondParameter)
{
    static std::vector<std::int8_t> value;
    return value;
}

const std::vector<std::int16_t>& CompileTestFunctions::arrayRefInt16Method(const std::vector<std::int16_t>& firstParameter, const std::vector<std::int16_t>& secondParameter)
{
    static std::vector<std::int16_t> value;
    return value;
}

const std::vector<std::int32_t>& CompileTestFunctions::arrayRefInt32Method(const std::vector<std::int32_t>& firstParameter, const std::vector<std::int32_t>& secondParameter)
{
    static std::vector<std::int32_t> value;
    return value;
}

const std::vector<std::int64_t>& CompileTestFunctions::arrayRefInt64Method(const std::vector<std::int64_t>& firstParameter, const std::vector<std::int64_t>& secondParameter)
{
    static std::vector<std::int64_t> value;
    return value;
}

const std::vector<std::uint8_t>& CompileTestFunctions::arrayRefUnsignedInt8Method(const std::vector<std::uint8_t>& firstParameter, const std::vector<std::uint8_t>& secondParameter)
{
    static std::vector<std::uint8_t> value;
    return value;
}

const std::vector<std::uint16_t>& CompileTestFunctions::arrayRefUnsignedInt16Method(const std::vector<std::uint16_t>& firstParameter, const std::vector<std::uint16_t>& secondParameter)
{
    static std::vector<std::uint16_t> value;
    return value;
}

const std::vector<std::uint32_t>& CompileTestFunctions::arrayRefUnsignedInt32Method(const std::vector<std::uint32_t>& firstParameter, const std::vector<std::uint32_t>& secondParameter)
{
    static std::vector<std::uint32_t> value;
    return value;
}

const std::vector<std::uint64_t>& CompileTestFunctions::arrayRefUnsignedInt64Method(const std::vector<std::uint64_t>& firstParameter, const std::vector<std::uint64_t>& secondParameter)
{
    static std::vector<std::uint64_t> value;
    return value;
}

const std::vector<float>& CompileTestFunctions::arrayRefFloatMethod(const std::vector<float>& firstParameter, const std::vector<float>& secondParameter)
{
    static std::vector<float> value;
    return value;
}

const std::vector<double>& CompileTestFunctions::arrayRefDoubleMethod(const std::vector<double>& firstParameter, const std::vector<double>& secondParameter)
{
    static std::vector<double> value;
    return value;
}

const std::vector<bool>& CompileTestFunctions::arrayRefBoolMethod(const std::vector<bool>& firstParameter, const std::vector<bool>& secondParameter)
{
    static std::vector<bool> value;
    return value;
}

const std::vector<std::string>& CompileTestFunctions::arrayRefStringMethod(const std::vector<std::string>& firstParameter, const std::vector<std::string>& secondParameter)
{
    static std::vector<std::string> value;
    return value;
}

const std::vector<CompileTestExampleEnum>& CompileTestFunctions::arrayRefEnumMethod(const std::vector<CompileTestExampleEnum>& firstParameter, const std::vector<CompileTestExampleEnum>& secondParameter)
{
    static std::vector<CompileTestExampleEnum> value;
    return value;
}

const std::vector<CompileTestDummyClass>& CompileTestFunctions::arrayRefClassMethod(const std::vector<CompileTestDummyClass>& firstParameter, const std::vector<CompileTestDummyClass>& secondParameter)
{
    static std::vector<CompileTestDummyClass> value;
    return value;
}

std::vector<std::shared_ptr<CompileTestDummyClass>> CompileTestFunctions::arrayPtrClassMethod(std::vector<std::shared_ptr<CompileTestDummyClass>> firstParameter, std::vector<std::shared_ptr<CompileTestDummyClass>> secondParameter)
{
    return {};
}

const std::vector<std::shared_ptr<CompileTestDummyClass>>& CompileTestFunctions::arrayPtrRefClassMethod(const std::vector<std::shared_ptr<CompileTestDummyClass>>& firstParameter, const std::vector<std::shared_ptr<CompileTestDummyClass>>& secondParameter)
{
    static std::vector<std::shared_ptr<CompileTestDummyClass>> value;
    return value;
}

const std::uint8_t* CompileTestFunctions::dataMethod(const std::uint8_t* firstParameter, std::size_t firstParameterSize, const std::uint8_t* secondParameter, std::size_t secondParameterSize, std::size_t& outSize)
{
    outSize = 0;
    return nullptr;
}

}
