import { api } from "../httpClient";

export const fetchImportedPortfolio = async (id) => {
  const url = `/import/portfolio/${id}`;
  try {
    const response = await api.get(url);

    if (response.status !== 200) {
      console.error(
        `Failed to fetch imported portfolio (${id}): {status: ${response.status}, data: ${response.data}}`
      );
      return null;
    }

    return response.data;
  } catch (error) {
    console.error(error);
    return null;
  }
};
