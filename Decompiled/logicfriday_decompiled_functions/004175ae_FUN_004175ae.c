/* 004175ae FUN_004175ae */

bool __thiscall FUN_004175ae(void *this,uint param_1,uint param_2)

{
  if (param_2 > param_1) {
    MessageBoxA(*(HWND *)this,
                "Insufficient Length:\n\nThe number of bits in <unsigned int> cannot be less than the count\n of input variables in the logic function."
                ,"Function Creation Failed",0);
  }
  return param_2 <= param_1;
}
