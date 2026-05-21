/* 0043f086 FUN_0043f086 */

undefined4 * __cdecl FUN_0043f086(undefined4 *param_1,undefined4 param_2)

{
  _ptiddata p_Var1;
  
  *param_1 = param_2;
  p_Var1 = __getptd();
  param_1[1] = p_Var1->_purecall;
  p_Var1 = __getptd();
  p_Var1->_purecall = param_1;
  return param_1;
}
